use mundam_runtime_probe::{MAX_MODULE_BYTES, Outcome, supervised::run};
use std::{
    future, io,
    path::Path,
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{io::AsyncWriteExt, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SIMPLE: &str = "(module (func (export \"run\") (result i32) i32.const 42))";

fn helper_path() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_mundam-runtime-probe"))
}

async fn execute(directory: &Path, fixture: &str) -> Result<Outcome, Box<dyn std::error::Error>> {
    let bytes = wat::parse_str(fixture)?;
    Ok(run(
        helper_path(),
        directory,
        &bytes,
        Duration::from_secs(2),
        future::pending(),
    )
    .await?)
}

#[tokio::test]
async fn runs_an_import_free_module_in_the_helper() -> TestResult {
    let directory = tempfile::tempdir()?;
    let module = wat::parse_str(SIMPLE)?;
    let mut samples = [Duration::ZERO; 11];
    for elapsed in &mut samples {
        let start = Instant::now();
        assert_eq!(
            run(
                helper_path(),
                directory.path(),
                &module,
                Duration::from_secs(2),
                future::pending()
            )
            .await?,
            Outcome::Completed(42)
        );
        *elapsed = start.elapsed();
    }
    samples.sort_unstable();
    eprintln!(
        "11 independent helper launches, median {:?}, maximum {:?}",
        samples[5], samples[10]
    );
    Ok(())
}

#[tokio::test]
async fn rejects_imports_start_functions_and_incompatible_exports() -> TestResult {
    let directory = tempfile::tempdir()?;
    assert_eq!(
        execute(
            directory.path(),
            r#"(module (import "wasi_snapshot_preview1" "fd_write" (func))
          (func (export "run") (result i32) i32.const 1))"#
        )
        .await?,
        Outcome::ImportsDenied
    );
    for module in [
        r#"(module (func $init) (start $init) (func (export "run") (result i32) i32.const 42))"#,
        r#"(module (func (export "run") (result i64) i64.const 42))"#,
    ] {
        assert_eq!(
            execute(directory.path(), module).await?,
            Outcome::InvalidModule
        );
    }
    Ok(())
}

#[tokio::test]
async fn bounds_execution_memory_table_and_stack() -> TestResult {
    let directory = tempfile::tempdir()?;
    let loop_fixture = r#"(module (func (export "run") (result i32)
      (loop $repeat br $repeat) i32.const 0))"#;
    assert_eq!(
        execute(directory.path(), loop_fixture).await?,
        Outcome::FuelExhausted
    );
    for module in [
        r#"(module (memory 3) (func (export "run") (result i32) i32.const 0))"#,
        r#"(module (memory 1) (func (export "run") (result i32) i32.const 2 memory.grow))"#,
        r#"(module (table 33 funcref) (func (export "run") (result i32) i32.const 0))"#,
        r#"(module (func $entry (export "run") (result i32) call $entry))"#,
    ] {
        assert_eq!(
            execute(directory.path(), module).await?,
            Outcome::ResourceLimit,
            "{module}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn stops_on_deadline_or_cancellation_and_accepts_the_next_request() -> TestResult {
    let directory = tempfile::tempdir()?;
    let module = wat::parse_str(SIMPLE)?;
    assert_eq!(
        run(
            helper_path(),
            directory.path(),
            &module,
            Duration::ZERO,
            future::pending()
        )
        .await?,
        Outcome::TimedOut
    );
    assert_eq!(
        run(
            helper_path(),
            directory.path(),
            &module,
            Duration::from_secs(2),
            future::ready(())
        )
        .await?,
        Outcome::Cancelled
    );
    assert_eq!(
        execute(directory.path(), SIMPLE).await?,
        Outcome::Completed(42)
    );
    Ok(())
}

#[tokio::test]
async fn rejects_oversized_and_malformed_modules() -> TestResult {
    let directory = tempfile::tempdir()?;
    let oversized = vec![0; MAX_MODULE_BYTES + 1];
    let result = run(
        helper_path(),
        directory.path(),
        &oversized,
        Duration::from_secs(2),
        future::pending(),
    )
    .await;
    assert_eq!(
        result.err().ok_or("accepted oversized request")?.kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        run(
            helper_path(),
            directory.path(),
            b"invalid",
            Duration::from_secs(2),
            future::pending()
        )
        .await?,
        Outcome::InvalidModule
    );
    Ok(())
}

#[tokio::test]
async fn child_rejects_oversized_headers_without_waiting_for_the_body() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut command = Command::new(helper_path());
    command
        .current_dir(directory.path())
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    let mut child = command.spawn()?;
    let mut input = child.stdin.take().ok_or("missing child input")?;
    input
        .write_all(&((MAX_MODULE_BYTES as u32) + 1).to_le_bytes())
        .await?;
    // Keep the pipe open until the child exits so an accidental body read stalls the test.
    let status = tokio::time::timeout(Duration::from_secs(2), child.wait()).await??;
    drop(input);
    assert!(!status.success());
    Ok(())
}
