use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::process::Command;
use tokio::sync::Notify;

#[test]
fn owned_process_fixture() {
    if std::env::var_os("MUNDAM_HLS_TEST_CHILD").is_some() {
        std::thread::sleep(Duration::from_secs(60));
    }
}

fn child() -> AppResult<Child> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args([
            "--exact",
            "feature::transcoding::hls_manager::tests::owned_process_fixture",
        ])
        .env("MUNDAM_HLS_TEST_CHILD", "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    Ok(command.spawn()?)
}

#[tokio::test]
async fn session_waits_for_its_child_before_removing_files() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let output_dir = directory.path().join("owned");
    tokio::fs::create_dir_all(&output_dir).await?;
    let mut session = StreamSession {
        process: Some(child()?),
        last_accessed: Instant::now(),
        output_dir: output_dir.clone(),
    };
    tokio::time::timeout(Duration::from_secs(5), session.stop())
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    assert!(session.process.is_none());
    assert!(!output_dir.exists());
    session.stop().await?;
    Ok(())
}

#[tokio::test]
async fn concurrent_requests_share_one_child_and_shutdown_refuses_late_requests() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let original = directory.path().join("original");
    tokio::fs::write(&original, b"unchanged").await?;
    let cancellation = CancellationToken::new();
    let manager = HlsManager::new(directory.path(), cancellation.clone());
    let calls = Arc::new(AtomicUsize::new(0));
    let owner = manager.clone();
    let count = calls.clone();
    let worker = tokio::spawn(async move {
        owner
            .run_with_factory(
                Duration::from_secs(90),
                Duration::from_secs(10),
                move |_, _, _| {
                    count.fetch_add(1, Ordering::SeqCst);
                    async { child() }
                },
            )
            .await
    });
    let (first, second) = tokio::join!(
        manager.get_or_start_stream("../original", &original, None),
        manager.get_or_start_stream("../original", &original, None)
    );
    let first = first?;
    assert_eq!(first, second?);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    cancellation.cancel();
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .map_err(|error| AppError::Internal(error.to_string()))??;
    assert!(!first.exists());
    assert_eq!(tokio::fs::read(&original).await?, b"unchanged");
    assert!(matches!(
        manager.get_or_start_stream("late", &original, None).await,
        Err(AppError::Cancelled)
    ));
    Ok(())
}

#[tokio::test]
async fn dropped_request_and_shutdown_keep_the_admitted_setup_owned() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let cancellation = CancellationToken::new();
    let manager = HlsManager::new(directory.path(), cancellation.clone());
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let calls = Arc::new(AtomicUsize::new(0));
    let owner = manager.clone();
    let owner_started = started.clone();
    let owner_release = release.clone();
    let owner_calls = calls.clone();
    let worker = tokio::spawn(async move {
        owner
            .run_with_factory(
                Duration::from_secs(90),
                Duration::from_secs(10),
                move |_, _, _| {
                    owner_calls.fetch_add(1, Ordering::SeqCst);
                    let started = owner_started.clone();
                    let release = owner_release.clone();
                    async move {
                        started.notify_one();
                        release.notified().await;
                        child()
                    }
                },
            )
            .await
    });
    let requester = manager.clone();
    let request = tokio::spawn(async move {
        requester
            .get_or_start_stream("first", Path::new("unused"), None)
            .await
    });
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    request.abort();
    assert!(request.await.is_err());
    let mut queued = Vec::new();
    // Fill the queue and leave one sender backpressured while setup remains owned.
    for index in 0..33 {
        let queued_manager = manager.clone();
        queued.push(tokio::spawn(async move {
            queued_manager
                .get_or_start_stream(&format!("queued-{index}"), Path::new("unused"), None)
                .await
        }));
    }
    tokio::time::timeout(Duration::from_secs(3), async {
        while manager.commands.capacity() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;
    cancellation.cancel();
    assert!(!worker.is_finished());
    release.notify_one();
    for request in queued {
        assert!(matches!(
            request
                .await
                .map_err(|error| AppError::Internal(error.to_string()))?,
            Err(AppError::Cancelled)
        ));
    }
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .map_err(|error| AppError::Internal(error.to_string()))??;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(std::fs::read_dir(&manager.streams_dir)?.count(), 0);
    Ok(())
}

#[tokio::test]
async fn cancelled_owner_starts_no_child_or_directory() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let cancellation = CancellationToken::new();
    let manager = HlsManager::new(directory.path(), cancellation.clone());
    cancellation.cancel();
    manager
        .run_with_factory(
            Duration::from_secs(90),
            Duration::from_secs(10),
            |_, _, _| async {
                panic!("cancelled owner must not start a child");
            },
        )
        .await?;
    assert!(!manager.streams_dir.exists());
    assert!(matches!(
        manager
            .get_or_start_stream("late", Path::new("unused"), None)
            .await,
        Err(AppError::Cancelled)
    ));
    Ok(())
}

#[tokio::test]
async fn cleanup_failure_is_reported_instead_of_clean_shutdown() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let cancellation = CancellationToken::new();
    let manager = HlsManager::new(directory.path(), cancellation.clone());
    let owner = manager.clone();
    let worker = tokio::spawn(async move {
        owner
            .run_with_factory(
                Duration::from_secs(90),
                Duration::from_secs(10),
                |_, output_dir, _| async move {
                    tokio::fs::remove_dir(&output_dir).await?;
                    tokio::fs::write(output_dir, b"cleanup error fixture").await?;
                    child()
                },
            )
            .await
    });
    manager
        .get_or_start_stream("failure", Path::new("unused"), None)
        .await?;
    cancellation.cancel();
    let result = tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .map_err(|error| AppError::Internal(error.to_string()))?;
    assert!(result.is_err());
    Ok(())
}

#[tokio::test]
async fn expiration_waits_and_removes_only_the_retired_session() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let cancellation = CancellationToken::new();
    let manager = HlsManager::new(directory.path(), cancellation.clone());
    let owner = manager.clone();
    let worker = tokio::spawn(async move {
        owner
            .run_with_factory(Duration::ZERO, Duration::from_millis(10), |_, _, _| async {
                child()
            })
            .await
    });
    let first = manager
        .get_or_start_stream("same", Path::new("unused"), None)
        .await?;
    tokio::time::timeout(Duration::from_secs(5), async {
        while first.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;
    let replacement = manager
        .get_or_start_stream("same", Path::new("unused"), None)
        .await?;
    assert_ne!(first, replacement);
    cancellation.cancel();
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .map_err(|error| AppError::Internal(error.to_string()))??;
    assert!(!replacement.exists());
    Ok(())
}

#[tokio::test]
async fn production_ffmpeg_session_creates_playlist_and_is_drained() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let original = directory.path().join("fixture.wav");
    let fixture_path = original.clone();
    tokio::task::spawn_blocking(move || {
        let tools = crate::processing::transcoding::resolve_transcoding_tools::<tauri::Wry>(None)?;
        let mut command = std::process::Command::new(tools.ffmpeg);
        command
            .args([
                "-f",
                "lavfi",
                "-i",
                "anullsrc=r=48000:cl=stereo",
                "-t",
                "2",
                "-c:a",
                "pcm_s16le",
            ])
            .arg(fixture_path);
        let output = crate::processing::transcoding::run_command_with_timeout(command, 30)?;
        if !output.status.success() {
            return Err(AppError::Transcoding(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
        Ok(())
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))??;
    let original_bytes = tokio::fs::read(&original).await?;
    let cancellation = CancellationToken::new();
    let manager = HlsManager::new(directory.path(), cancellation.clone());
    let owner = manager.clone();
    let worker = tokio::spawn(async move { owner.run(90).await });
    let observed: AppResult<_> = async {
        let output = manager
            .get_or_start_stream("fixture", &original, Some("audio/wav"))
            .await?;
        let playlist = output.join("playlist.m3u8");
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                if let Ok(text) = tokio::fs::read_to_string(&playlist).await {
                    if text.contains("#EXTM3U") {
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .map_err(|error| {
            AppError::Internal(format!("HLS fixture playlist was not generated: {error}"))
        })?;
        Ok(output)
    }
    .await;
    cancellation.cancel();
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .map_err(|error| AppError::Internal(error.to_string()))??;
    let output = observed?;
    assert!(!output.exists());
    assert_eq!(tokio::fs::read(&original).await?, original_bytes);
    Ok(())
}
