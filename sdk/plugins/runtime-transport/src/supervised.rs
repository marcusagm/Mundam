//! Runs the trusted probe executable with one request and bounded replies.

use crate::{
    COMPONENT_HEADER, MAX_MODULE_BYTES, Outcome, decode,
    preview::{
        COMPONENT_MODE, COMPONENT_RESULT, FINISHED, MODE, PreviewBudget, PreviewRead, READ, READ_OK,
    },
};
use std::{future::Future, io, path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, ChildStdout, Command},
    time::{Instant, sleep_until},
};

const MAX_WALL_TIME: Duration = Duration::from_secs(2);

/// Runs one trusted helper process in a caller-owned working directory.
///
/// # Errors
/// Rejects invalid request paths, launch or containment failures, child exit failures and
/// malformed replies. A rejected request cannot leave a running helper owned by this call.
pub async fn run(
    executable: &Path,
    working_directory: &Path,
    module: &[u8],
    wall_time: Duration,
    cancel: impl Future<Output = ()>,
) -> io::Result<Outcome> {
    supervise(
        executable,
        working_directory,
        module,
        wall_time,
        cancel,
        async |child| exchange(child, module).await,
    )
    .await
}

/// Runs a probe with one host-scoped preview and a callback for each bounded read.
///
/// # Errors
/// Rejects invalid framing, failed preview reads, invalid callback responses and
/// launch or child failures. A cancelled read or pipe operation is dropped before
/// the supervisor terminates and reaps the helper.
pub async fn run_with_preview(
    executable: &Path,
    working_directory: &Path,
    module: &[u8],
    byte_count: u32,
    wall_time: Duration,
    cancel: impl Future<Output = ()>,
    read: impl AsyncFnMut(PreviewRead) -> io::Result<Vec<u8>>,
) -> io::Result<Outcome> {
    let budget = PreviewBudget::new(byte_count)?;
    supervise(
        executable,
        working_directory,
        module,
        wall_time,
        cancel,
        async move |child| exchange_preview(child, module, byte_count, budget, read).await,
    )
    .await
}

async fn supervise(
    executable: &Path,
    working_directory: &Path,
    module: &[u8],
    wall_time: Duration,
    cancel: impl Future<Output = ()>,
    exchange: impl AsyncFnOnce(&mut Child) -> io::Result<Outcome>,
) -> io::Result<Outcome> {
    if !executable.is_absolute()
        || !working_directory.is_absolute()
        || module.len() > MAX_MODULE_BYTES
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid probe request",
        ));
    }
    let deadline = Instant::now() + wall_time.min(MAX_WALL_TIME);
    let mut command = Command::new(executable);
    command
        .current_dir(working_directory)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    let mut child = command.spawn()?;
    // The helper waits for its first frame before parsing any guest bytes.
    #[cfg(windows)]
    let _job = match crate::job::attach(&child) {
        Ok(job) => job,
        Err(error) => {
            stop(&mut child).await?;
            return Err(error);
        }
    };
    let result = if Instant::now() >= deadline {
        Ok(Outcome::TimedOut)
    } else {
        tokio::select! {
            biased;
            _ = cancel => Ok(Outcome::Cancelled),
            _ = sleep_until(deadline) => Ok(Outcome::TimedOut),
            result = exchange(&mut child) => result,
        }
    };
    if matches!(&result, Err(_) | Ok(Outcome::Cancelled | Outcome::TimedOut)) {
        stop(&mut child).await?;
    }
    result
}

async fn stop(child: &mut Child) -> io::Result<()> {
    if child.try_wait()?.is_none() {
        child.kill().await?;
    }
    child.wait().await?;
    Ok(())
}

async fn exchange(child: &mut Child, module: &[u8]) -> io::Result<Outcome> {
    let mut input = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("missing request pipe"))?;
    let mut output = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing reply pipe"))?;
    input
        .write_all(&(module.len() as u32).to_le_bytes())
        .await?;
    input.write_all(module).await?;
    drop(input);
    finish(child, &mut output, false).await
}

async fn exchange_preview(
    child: &mut Child,
    module: &[u8],
    byte_count: u32,
    mut budget: PreviewBudget,
    mut read: impl AsyncFnMut(PreviewRead) -> io::Result<Vec<u8>>,
) -> io::Result<Outcome> {
    let mut input = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("missing request pipe"))?;
    let mut output = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing reply pipe"))?;
    let component = module.starts_with(COMPONENT_HEADER);
    input
        .write_all(if component { &COMPONENT_MODE } else { &MODE })
        .await?;
    input
        .write_all(&(module.len() as u32).to_le_bytes())
        .await?;
    input.write_all(&byte_count.to_le_bytes()).await?;
    input.write_all(module).await?;
    loop {
        let tag = output.read_u8().await?;
        match tag {
            FINISHED => {
                drop(input);
                return finish(child, &mut output, component).await;
            }
            COMPONENT_RESULT if component => {
                drop(input);
                let mut signature = [0; 8];
                output.read_exact(&mut signature).await?;
                await_exit(child, &mut output).await?;
                return Ok(Outcome::ComponentCompleted(signature));
            }
            READ => {
                let mut frame = [0; 8];
                output.read_exact(&mut frame).await?;
                let request = PreviewRead::from_frame(frame);
                budget.reserve(request)?;
                let bytes = read(request).await?;
                if bytes.len() != request.length as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "incorrect preview reply length",
                    ));
                }
                input.write_all(&[READ_OK]).await?;
                input.write_all(&bytes).await?;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unknown helper message",
                ));
            }
        }
    }
}

async fn finish(
    child: &mut Child,
    output: &mut ChildStdout,
    component: bool,
) -> io::Result<Outcome> {
    let mut reply = [0; 5];
    output.read_exact(&mut reply).await?;
    await_exit(child, output).await?;
    decode(reply, component)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid child outcome"))
}

async fn await_exit(child: &mut Child, output: &mut ChildStdout) -> io::Result<()> {
    if output.read(&mut [0]).await? != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unexpected child output",
        ));
    }
    if !child.wait().await?.success() {
        return Err(io::Error::other("probe process failed"));
    }
    Ok(())
}
