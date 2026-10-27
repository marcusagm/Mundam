//! Orquestração de subprocessos para FFmpeg e FFprobe.
//!
//! Este módulo garante que comandos de CLI sejam executados de forma segura,
//! com limites de tempo (timeouts) e limpeza de processos órfãos.
use crate::core::error::{AppError, AppResult};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use tauri::Manager;
use wait_timeout::ChildExt;

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
        .ok_or_else(|| AppError::Transcoding("FFmpeg binary not found".to_string()))?;
    let ffprobe = find_binary(app_handle, "ffprobe")
        .ok_or_else(|| AppError::Transcoding("FFprobe binary not found".to_string()))?;
    let assimp = find_binary(app_handle, "assimp");

    Ok(TranscodingTools {
        ffmpeg,
        ffprobe,
        assimp,
    })
}

/// Tenta encontrar um binário específico no sistema.
fn find_binary<R: tauri::Runtime>(
    app_handle: Option<&tauri::AppHandle<R>>,
    name: &str,
) -> Option<PathBuf> {
    let binary_name = if cfg!(target_os = "windows") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };

    let platform_dir = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows-x64" // Default para x64, pode ser refinado
    } else {
        "linux"
    };

    // 1. Tenta via Tauri Resource Dir (Bundled)
    if let Some(handle) = app_handle {
        if let Ok(resource_dir) = handle.path().resource_dir() {
            // Tenta caminho direto: resource/name/binary
            let direct_path = resource_dir.join(name).join(&binary_name);
            if direct_path.exists() {
                return Some(direct_path);
            }

            // Tenta caminho por plataforma: resource/name/platform/binary
            let platform_path = resource_dir
                .join(name)
                .join(platform_dir)
                .join(&binary_name);
            if platform_path.exists() {
                return Some(platform_path);
            }

            // Tenta caminho por plataforma com bin/ (comum em assimp macos): resource/name/platform/bin/binary
            let bin_path = resource_dir
                .join(name)
                .join(platform_dir)
                .join("bin")
                .join(&binary_name);
            if bin_path.exists() {
                return Some(bin_path);
            }
        }
    }

    // 2. Tenta via caminhos relativos de desenvolvimento (src-tauri/...)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(target_dir) = exe_path.parent() {
            if let Some(debug_dir) = target_dir.parent() {
                if let Some(src_tauri) = debug_dir.parent() {
                    // Tenta caminho direto
                    let direct_path = src_tauri.join(name).join(&binary_name);
                    if direct_path.exists() {
                        return Some(direct_path);
                    }

                    // Tenta caminho por plataforma
                    let platform_path = src_tauri.join(name).join(platform_dir).join(&binary_name);
                    if platform_path.exists() {
                        return Some(platform_path);
                    }

                    // Tenta caminho por plataforma com bin/
                    let bin_path = src_tauri
                        .join(name)
                        .join(platform_dir)
                        .join("bin")
                        .join(&binary_name);
                    if bin_path.exists() {
                        return Some(bin_path);
                    }
                }
            }
        }
    }

    // 3. Tenta via PATH global do sistema
    let mut probe_cmd = Command::new(name);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        probe_cmd.creation_flags(0x08000000);
    }
    if probe_cmd
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some(PathBuf::from(name));
    }
    None
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
    mut command: Command,
    timeout_seconds: u64,
) -> AppResult<std::process::Output> {
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
        .map_err(|error| AppError::Transcoding(format!("Failed to spawn process: {}", error)))?;

    let mut stdout_pipe = child_process
        .stdout
        .take()
        .ok_or_else(|| AppError::Transcoding("Failed to open stdout pipe".to_string()))?;
    let mut stderr_pipe = child_process
        .stderr
        .take()
        .ok_or_else(|| AppError::Transcoding("Failed to open stderr pipe".to_string()))?;

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

    match child_process.wait_timeout(Duration::from_secs(timeout_seconds))? {
        Some(exit_status) => {
            let stdout_data = stdout_join_handle
                .join()
                .map_err(|_| AppError::Transcoding("Stdout reader thread panicked".to_string()))?
                .map_err(|error| {
                    AppError::Transcoding(format!("Failed to read stdout: {}", error))
                })?;
            let stderr_data = stderr_join_handle
                .join()
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
        None => {
            // Process timed out. Kill it to prevent zombies.
            child_process.kill().ok();
            let _ = stdout_join_handle.join();
            let _ = stderr_join_handle.join();
            Err(AppError::Transcoding(format!(
                "Process execution timed out after {}s",
                timeout_seconds
            )))
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
