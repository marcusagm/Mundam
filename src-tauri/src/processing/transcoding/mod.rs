//! Orquestração de subprocessos para FFmpeg e FFprobe.
//!
//! Este módulo garante que comandos de CLI sejam executados de forma segura,
//! com limites de tempo (timeouts) e limpeza de processos órfãos.
use crate::core::error::{AppError, AppResult};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
pub mod binaries;
use wait_timeout::ChildExt;

static H264_ENCODERS: OnceLock<Mutex<HashMap<PathBuf, &'static str>>> = OnceLock::new();

/// Finds a usable H.264 encoder in the selected FFmpeg binary and caches the result.
pub fn resolve_h264_encoder(ffmpeg: &Path) -> AppResult<&'static str> {
    let cache = H264_ENCODERS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(encoder) = cache
        .lock()
        .map_err(|_| AppError::Transcoding("H.264 encoder cache is unavailable".into()))?
        .get(ffmpeg)
        .copied()
    {
        return Ok(encoder);
    }

    let output = Command::new(ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .output()
        .map_err(|error| {
            AppError::Transcoding(format!("Failed to list FFmpeg encoders: {error}"))
        })?;
    if !output.status.success() {
        return Err(AppError::Transcoding(
            "FFmpeg could not list its available encoders".into(),
        ));
    }

    let encoder_list = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let available: Vec<&str> = encoder_list
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .collect();
    let encoder = [
        "libx264",
        "libopenh264",
        "h264_mf",
        "h264_videotoolbox",
        "h264_nvenc",
        "h264_amf",
        "h264_qsv",
    ]
    .into_iter()
    .find(|candidate| available.contains(candidate))
    .ok_or_else(|| {
        AppError::Transcoding("No supported H.264 encoder is available in FFmpeg".into())
    })?;

    cache
        .lock()
        .map_err(|_| AppError::Transcoding("H.264 encoder cache is unavailable".into()))?
        .insert(ffmpeg.to_path_buf(), encoder);
    Ok(encoder)
}

/// Caminhos resolvidos para os binários de vídeo.
pub struct TranscodingTools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub assimp: Option<PathBuf>,
}

/// Resolve os caminhos para as ferramentas de transcoding.
///
/// Tenta localizar os binários na pasta de recursos do Tauri, no diretório de debug
/// ou no PATH global do sistema.
pub fn resolve_transcoding_tools<R: tauri::Runtime>(
    app_handle: Option<&tauri::AppHandle<R>>,
) -> AppResult<TranscodingTools> {
    let ffmpeg = find_binary(app_handle, "ffmpeg")
        .ok_or_else(|| AppError::ToolUnavailable("ffmpeg".into()))?;
    let ffprobe = find_binary(app_handle, "ffprobe")
        .ok_or_else(|| AppError::ToolUnavailable("ffprobe".into()))?;
    let assimp = find_binary(app_handle, "assimp");

    Ok(TranscodingTools {
        ffmpeg,
        ffprobe,
        assimp,
    })
}

fn find_binary<R: tauri::Runtime>(
    handle: Option<&tauri::AppHandle<R>>,
    name: &str,
) -> Option<PathBuf> {
    binaries::resolve(handle, name)
}

/// Resolves one independent extraction dependency, without requiring unrelated tools.
/// # Errors
/// Returns a typed unavailability error if discovery or a recent probe rejected the tool.
pub fn require_extraction_tool(
    tool: crate::core::processing::tools::ExtractionTool,
) -> AppResult<PathBuf> {
    find_binary::<tauri::Wry>(None, tool.as_str())
        .ok_or_else(|| AppError::ToolUnavailable(tool.as_str().into()))
}

/// Classifies missing or non-executable known binaries independently of asset corruption.
pub(crate) fn launch_error(program: &std::ffi::OsStr, error: std::io::Error) -> AppError {
    let name = std::path::Path::new(program)
        .file_stem()
        .and_then(|name| name.to_str());
    if matches!(
        error.kind(),
        std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
    ) {
        if let Some(name @ ("ffmpeg" | "ffprobe" | "assimp")) = name {
            binaries::unavailable(name);
            return AppError::ToolUnavailable(name.into());
        }
    }
    AppError::Io(error)
}

/// Executa um comando com um timeout rigoroso.
///
/// # Arguments
/// * `command` - O comando pronto para execução.
/// * `timeout_seconds` - Tempo máximo de execução em segundos.
///
/// # Errors
/// Retorna `AppError::Transcoding` se o comando falhar, expirar ou emitir erro.
pub fn run_command_with_timeout(
    command: Command,
    timeout_seconds: u64,
) -> AppResult<std::process::Output> {
    run_command_waiting(command, timeout_seconds, |child, timeout| {
        child.wait_timeout(timeout)
    })
}

fn run_command_waiting<Wait>(
    mut command: Command,
    timeout_seconds: u64,
    wait: Wait,
) -> AppResult<std::process::Output>
where
    Wait: FnOnce(
        &mut std::process::Child,
        Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>>,
{
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let mut child_process = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| launch_error(command.get_program(), error))?;

    let (Some(mut stdout_pipe), Some(mut stderr_pipe)) =
        (child_process.stdout.take(), child_process.stderr.take())
    else {
        let _ = child_process.kill();
        let _ = child_process.wait();
        return Err(AppError::Transcoding("Failed to open process pipes".into()));
    };

    let stdout_join_handle = std::thread::spawn(move || {
        let mut stdout_buffer = Vec::new();
        stdout_pipe
            .read_to_end(&mut stdout_buffer)
            .map(|_| stdout_buffer)
    });

    let stderr_join_handle = std::thread::spawn(move || {
        let mut stderr_buffer = Vec::new();
        stderr_pipe
            .read_to_end(&mut stderr_buffer)
            .map(|_| stderr_buffer)
    });

    match wait(&mut child_process, Duration::from_secs(timeout_seconds)) {
        Ok(Some(exit_status)) => {
            // Observe both readers before propagating an error from either one.
            let stdout_result = stdout_join_handle.join();
            let stderr_result = stderr_join_handle.join();
            let stdout_data = stdout_result
                .map_err(|_| AppError::Transcoding("Stdout reader thread panicked".to_string()))?
                .map_err(|error| {
                    AppError::Transcoding(format!("Failed to read stdout: {}", error))
                })?;
            let stderr_data = stderr_result
                .map_err(|_| AppError::Transcoding("Stderr reader thread panicked".to_string()))?
                .map_err(|error| {
                    AppError::Transcoding(format!("Failed to read stderr: {}", error))
                })?;

            Ok(std::process::Output {
                status: exit_status,
                stdout: stdout_data,
                stderr: stderr_data,
            })
        }
        wait_result => {
            // std::process::Child has no kill-on-drop; wait errors also require cleanup.
            child_process.kill().ok();
            let _ = child_process.wait();
            let _ = stdout_join_handle.join();
            let _ = stderr_join_handle.join();
            match wait_result {
                Err(error) => Err(error.into()),
                _ => Err(AppError::Transcoding(format!(
                    "Process execution timed out after {}s",
                    timeout_seconds
                ))),
            }
        }
    }
}

/// Verifica a disponibilidade dos binários de transcoding.
pub fn check_transcoding_availability() -> bool {
    // Verificação rápida apenas de existência estática
    find_binary::<tauri::Wry>(None, "ffmpeg").is_some()
        && find_binary::<tauri::Wry>(None, "ffprobe").is_some()
}

/// Unit tests for the transcoding module.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_process_fixture() {
        if std::env::var_os("MUNDAM_NATIVE_TEST_CHILD").is_some() {
            std::thread::sleep(Duration::from_secs(60));
        }
    }

    fn fixture_command() -> std::io::Result<Command> {
        let mut command = Command::new(std::env::current_exe()?);
        command
            .args([
                "--exact",
                "processing::transcoding::tests::native_process_fixture",
            ])
            .env("MUNDAM_NATIVE_TEST_CHILD", "1");
        Ok(command)
    }

    #[test]
    fn native_wait_error_still_terminates_child_and_joins_both_pipes() -> AppResult<()> {
        let result = run_command_waiting(fixture_command()?, 30, |_, _| {
            Err(std::io::Error::other("wait failure fixture"))
        });
        assert!(result.is_err());
        assert!(result
            .err()
            .is_some_and(|error| error.to_string().contains("wait failure fixture")));
        Ok(())
    }

    #[test]
    fn native_timeout_terminates_child_and_joins_both_pipes() -> AppResult<()> {
        let result = run_command_with_timeout(fixture_command()?, 1);
        assert!(
            matches!(result, Err(AppError::Transcoding(message)) if message.contains("timed out"))
        );
        Ok(())
    }

    /// Testa a detecção de binários de transcoding.
    #[test]
    fn test_binary_detection() {
        // No CI/Local deve encontrar se instalado
        let tools = resolve_transcoding_tools::<tauri::Wry>(None);
        assert!(
            tools.is_ok(),
            "FFmpeg/FFprobe should be available for tests"
        );
    }

    /// Testa a execução de um comando com timeout.
    #[test]
    fn test_timeout_execution() {
        let mut cmd = Command::new("sleep");
        cmd.arg("2");
        // Deve falhar com timeout de 1s
        let result = run_command_with_timeout(cmd, 1);
        assert!(result.is_err());
    }
}
