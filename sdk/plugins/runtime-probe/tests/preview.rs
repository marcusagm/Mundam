use mundam_runtime_probe::{
    Outcome,
    preview::{PreviewBudget, PreviewRead},
    supervised::{run, run_with_preview},
};
use std::{cell::Cell, future, io, path::Path, time::Duration};
use tokio::sync::oneshot;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MODULE: &str = include_str!("fixtures/preview-read.wat");
const PREVIEW: &[u8] = b"RIFFtestWEBP";
const TEST_DEADLINE: Duration = Duration::from_secs(2);

fn helper_path() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_mundam-runtime-probe"))
}

#[tokio::test]
async fn reads_two_ranges_from_the_scoped_preview() -> TestResult {
    let directory = tempfile::tempdir()?;
    let module = wat::parse_str(MODULE)?;
    let mut requests = Vec::new();
    let outcome = run_with_preview(
        helper_path(),
        directory.path(),
        &module,
        PREVIEW.len() as u32,
        TEST_DEADLINE,
        future::pending(),
        async |request: PreviewRead| {
            requests.push(request);
            let start = request.offset as usize;
            Ok(PREVIEW[start..start + request.length as usize].to_vec())
        },
    )
    .await?;
    assert_eq!(outcome, Outcome::Completed(i32::from_le_bytes(*b"RIFF")));
    assert_eq!(
        requests,
        [
            PreviewRead {
                offset: 0,
                length: 4
            },
            PreviewRead {
                offset: 4,
                length: 4
            }
        ]
    );
    assert_eq!(
        run(
            helper_path(),
            directory.path(),
            &module,
            TEST_DEADLINE,
            future::pending()
        )
        .await?,
        Outcome::ImportsDenied,
    );
    Ok(())
}

#[tokio::test]
async fn rejects_unknown_imports_or_invalid_memory_and_ranges_before_reading() -> TestResult {
    let directory = tempfile::tempdir()?;
    for (source, expected) in [
        (
            MODULE.replace("mundam:probe/preview@0.1.0", "wasi_snapshot_preview1"),
            Outcome::ImportsDenied,
        ),
        (
            MODULE.replace("i32.const 16", "i32.const 65535"),
            Outcome::GuestTrap,
        ),
        (
            MODULE.replacen("i32.const 0", "i32.const -1", 1),
            Outcome::GuestTrap,
        ),
    ] {
        let module = wat::parse_str(source)?;
        let result = run_with_preview(
            helper_path(),
            directory.path(),
            &module,
            PREVIEW.len() as u32,
            TEST_DEADLINE,
            future::pending(),
            async |_| -> io::Result<Vec<u8>> { panic!("invalid guest request reached the broker") },
        )
        .await?;
        assert_eq!(result, expected);
    }
    Ok(())
}

#[tokio::test]
async fn rejects_failed_or_incomplete_broker_replies() -> TestResult {
    let directory = tempfile::tempdir()?;
    let module = wat::parse_str(MODULE)?;
    for kind in [io::ErrorKind::PermissionDenied, io::ErrorKind::InvalidData] {
        let result = run_with_preview(
            helper_path(),
            directory.path(),
            &module,
            PREVIEW.len() as u32,
            TEST_DEADLINE,
            future::pending(),
            async |_| {
                if kind == io::ErrorKind::PermissionDenied {
                    return Err(io::Error::from(kind));
                }
                Ok(vec![0; 3])
            },
        )
        .await;
        assert!(matches!(result, Err(error) if error.kind() == kind));
    }
    Ok(())
}

#[tokio::test]
async fn cancellation_drops_a_pending_broker_read() -> TestResult {
    struct PendingRead<'a>(&'a Cell<bool>);
    impl Drop for PendingRead<'_> {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    let directory = tempfile::tempdir()?;
    let module = wat::parse_str(MODULE)?;
    let dropped = Cell::new(false);
    let (notify, started) = oneshot::channel();
    let mut notify = Some(notify);
    let outcome = run_with_preview(
        helper_path(),
        directory.path(),
        &module,
        PREVIEW.len() as u32,
        TEST_DEADLINE,
        async {
            let _ = started.await;
        },
        async |_| {
            let _read = PendingRead(&dropped);
            let _ = notify
                .take()
                .ok_or_else(|| io::Error::other("unexpected second read"))?
                .send(());
            future::pending().await
        },
    )
    .await?;
    assert_eq!(outcome, Outcome::Cancelled);
    assert!(dropped.get());
    Ok(())
}

#[test]
fn bounds_aggregate_and_call_count_independently() -> TestResult {
    let mut sizes = PreviewBudget::new(1024 * 1024)?;
    let mut calls = PreviewBudget::new(1)?;
    for _ in 0..16 {
        sizes.reserve(PreviewRead {
            offset: 0,
            length: 64 * 1024,
        })?;
    }
    assert!(
        sizes
            .reserve(PreviewRead {
                offset: 0,
                length: 1
            })
            .is_err()
    );
    for _ in 0..32 {
        calls.reserve(PreviewRead {
            offset: 0,
            length: 1,
        })?;
    }
    assert!(
        calls
            .reserve(PreviewRead {
                offset: 0,
                length: 1
            })
            .is_err()
    );
    Ok(())
}
