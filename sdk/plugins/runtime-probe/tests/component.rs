#![cfg(feature = "component-proof")]

use mundam_runtime_probe::{Outcome, PreviewRejection, supervised::run_with_preview};
use std::{future, io, path::Path, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn component_cannot_import_an_unrelated_interface() -> TestResult {
    let component = wat::parse_str(r#"(component (import "unrelated" (instance)))"#)?;
    let working_directory = tempfile::tempdir()?;
    let result = run_with_preview(
        Path::new(env!("CARGO_BIN_EXE_mundam-runtime-probe")),
        working_directory.path(),
        &component,
        12,
        Duration::from_secs(2),
        future::pending(),
        async |_| Err(io::Error::other("unexpected broker read")),
    )
    .await?;
    assert_eq!(result, Outcome::ImportsDenied);
    Ok(())
}

#[tokio::test]
#[ignore = "generate the WIT example component first"]
async fn component_range_error_travels_through_the_supervisor() -> TestResult {
    let component = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../v1/examples/wit-preview-check/target/wasm32-unknown-unknown/release/preview-check.component.wasm",
    ))?;
    let working_directory = tempfile::tempdir()?;
    let result = run_with_preview(
        Path::new(env!("CARGO_BIN_EXE_mundam-runtime-probe")),
        working_directory.path(),
        &component,
        8,
        Duration::from_secs(2),
        future::pending(),
        async |_| Err(io::Error::other("short preview reached the broker")),
    )
    .await?;
    assert_eq!(
        result,
        Outcome::PreviewRejected(PreviewRejection::InvalidRange)
    );
    Ok(())
}
