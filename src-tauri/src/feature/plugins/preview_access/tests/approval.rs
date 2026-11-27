use super::*;
use crate::{
    core::plugins::{InstallApproval, PluginActivationReview, PublisherAnchor},
    infra::database::plugins::SqlitePluginRegistry,
};
use mundam_plugin_sdk::package::{sign_package, sign_worker_package, signing_key_fingerprint};
use serde_json::{json, Value};

const EXAMPLE_MANIFEST: &[u8] =
    include_bytes!("../../../../../../sdk/plugins/v1/examples/reference-review/manifest.json");
const EXAMPLE_ICON: &[u8] =
    include_bytes!("../../../../../../sdk/plugins/v1/examples/reference-review/assets/review.png");
const WORKER_SOURCE: &str =
    include_str!("../../../../../../sdk/plugins/runtime-probe/tests/fixtures/preview-read.wat");
const WORKER_ID: &str = "read-preview";

struct ApprovalFixture {
    registry: Arc<SqlitePluginRegistry>,
    service: PluginService,
    publisher: PublisherAnchor,
    review: PluginActivationReview,
    preview: Publisher,
}

impl ApprovalFixture {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let module = wat::parse_str(WORKER_SOURCE)?;
        Self::install(Some(("preview-probe-v1", &module))).await
    }

    async fn install(worker: Option<(&str, &[u8])>) -> Result<Self, Box<dyn std::error::Error>> {
        let preview = Publisher::new().await?;
        let registry = Arc::new(SqlitePluginRegistry::open(&registry_path(&preview)).await?);
        let service = PluginService::new(registry.clone());
        let mut key = vec![
            0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22,
            0x04, 0x20,
        ];
        key.extend_from_slice(&[29; 32]);
        let mut manifest: Value = serde_json::from_slice(EXAMPLE_MANIFEST)?;
        manifest["optionalPermissions"] = json!(["assetPreview.read"]);
        let publisher = PublisherAnchor {
            namespace: "org.example".into(),
            fingerprint: signing_key_fingerprint(&key)?,
        };
        service.trust_publisher(publisher.clone()).await?;
        let bytes = if let Some((abi, module)) = worker {
            manifest["worker"] = json!({"id": WORKER_ID, "abi": abi});
            sign_worker_package(&serde_json::to_vec(&manifest)?, EXAMPLE_ICON, module, &key)?
        } else {
            sign_package(&serde_json::to_vec(&manifest)?, EXAMPLE_ICON, &key)?
        };
        let package = service.preview(bytes.clone()).await?;
        assert_eq!(
            package.experimental_worker_id.as_deref(),
            worker.is_some().then_some(WORKER_ID)
        );
        service
            .install(
                bytes,
                InstallApproval {
                    package_digest: package.package_digest,
                },
            )
            .await?;
        let review = service
            .review_activation(
                preview.fixture.ledger.plugin_catalog_id().await?,
                &package.plugin_id,
            )
            .await?;
        Ok(Self {
            registry,
            service,
            publisher,
            review,
            preview,
        })
    }

    fn attempt(&self) -> PluginPreviewAttempt {
        PluginPreviewAttempt {
            plugin_id: self.review.plugin_id.clone(),
            worker_id: WORKER_ID.into(),
            package_digest: self.review.package_digest,
            selected_asset_id: "asset".into(),
            preview_revision: self.preview.revision,
            deadline: Instant::now() + MAX_LEASE_TIME,
            cancellation: CancellationToken::new(),
        }
    }

    async fn enable_preview(&self) -> AppResult<()> {
        self.service
            .enable(&self.review, vec![Permission::AssetPreviewRead])
            .await
    }

    async fn prepare(&self, attempt: PluginPreviewAttempt) -> AppResult<ApprovedPluginWorker> {
        self.service
            .prepare_signed_worker(self.preview.fixture.ledger.clone(), &self.preview, attempt)
            .await
    }
}

fn registry_path(preview: &Publisher) -> std::path::PathBuf {
    preview.fixture.data.path().join("plugins.sqlite")
}

#[async_trait]
impl PluginPreviewStorage for Publisher {
    async fn read_thumbnail(&self, file_name: &str) -> AppResult<Vec<u8>> {
        assert_eq!(file_name, self.cache_file);
        Ok(self.encoded.clone())
    }
}

#[tokio::test]
async fn plugin_preview_approval_requires_exact_package_permission_and_catalog() -> TestResult {
    let fixture = ApprovalFixture::new().await?;
    let other_catalog = fixture
        .service
        .review_activation("other-catalog".into(), &fixture.review.plugin_id)
        .await?;
    fixture
        .service
        .enable(&other_catalog, vec![Permission::AssetPreviewRead])
        .await?;
    fixture.service.enable(&fixture.review, vec![]).await?;
    assert!(matches!(
        fixture.prepare(fixture.attempt()).await,
        Err(AppError::Forbidden(_))
    ));

    fixture.enable_preview().await?;
    let mut attempt = fixture.attempt();
    attempt.package_digest[0] ^= 1;
    assert!(matches!(
        fixture.prepare(attempt).await,
        Err(AppError::Forbidden(_))
    ));
    let mut attempt = fixture.attempt();
    attempt.worker_id = "other-worker".into();
    assert!(matches!(
        fixture.prepare(attempt).await,
        Err(AppError::Forbidden(_))
    ));

    let safe_mode = PluginService::new(fixture.registry.clone()).with_safe_mode(true);
    assert!(matches!(
        safe_mode
            .prepare_signed_worker(
                fixture.preview.fixture.ledger.clone(),
                &fixture.preview,
                fixture.attempt()
            )
            .await,
        Err(AppError::Forbidden(_))
    ));
    let (module, mut preview) = fixture
        .prepare(fixture.attempt())
        .await?
        .admit_launch()
        .await?;
    assert_eq!(module, wat::parse_str(WORKER_SOURCE)?);
    let offer = preview.offer();
    assert_eq!(
        preview.read(offer.handle, 0, offer.byte_count).await?,
        fixture.preview.encoded
    );
    Ok(())
}

#[tokio::test]
async fn plugin_preview_approval_rejects_old_lease_after_reenable_and_revocation() -> TestResult {
    let fixture = ApprovalFixture::new().await?;
    fixture.enable_preview().await?;
    let prepared = fixture.prepare(fixture.attempt()).await?;
    let (_, mut old) = fixture
        .prepare(fixture.attempt())
        .await?
        .admit_launch()
        .await?;
    let other_registry =
        Arc::new(SqlitePluginRegistry::open(&registry_path(&fixture.preview)).await?);
    let other_service = PluginService::new(other_registry);
    other_service
        .disable(&fixture.review.catalog_id, &fixture.review.plugin_id)
        .await?;
    fixture.enable_preview().await?;
    assert!(matches!(
        prepared.admit_launch().await,
        Err(AppError::Forbidden(_))
    ));
    assert!(matches!(
        old.read(old.offer().handle, 0, 4).await,
        Err(AppError::Forbidden(_))
    ));

    let (_, mut current) = fixture
        .prepare(fixture.attempt())
        .await?
        .admit_launch()
        .await?;
    assert_eq!(current.read(current.offer().handle, 0, 4).await?, b"RIFF");
    other_service
        .revoke_publisher(fixture.publisher.clone())
        .await?;
    assert!(matches!(
        current.read(current.offer().handle, 0, 4).await,
        Err(AppError::Forbidden(_))
    ));
    Ok(())
}

struct DisableDuringRead<'a>(&'a ApprovalFixture);

#[async_trait]
impl PluginPreviewStorage for DisableDuringRead<'_> {
    async fn read_thumbnail(&self, file_name: &str) -> AppResult<Vec<u8>> {
        let fixture = self.0;
        tokio::time::timeout(
            Duration::from_secs(2),
            fixture
                .service
                .disable(&fixture.review.catalog_id, &fixture.review.plugin_id),
        )
        .await
        .map_err(|_| {
            AppError::Internal("Preview held registry lock during storage read".into())
        })??;
        fixture.preview.read_thumbnail(file_name).await
    }
}

#[tokio::test]
async fn plugin_preview_approval_is_checked_again_after_cache_io() -> TestResult {
    let fixture = ApprovalFixture::new().await?;
    fixture.enable_preview().await?;
    let result = fixture
        .service
        .prepare_signed_worker(
            fixture.preview.fixture.ledger.clone(),
            &DisableDuringRead(&fixture),
            fixture.attempt(),
        )
        .await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
    Ok(())
}

#[tokio::test]
async fn plugin_preview_approval_does_not_admit_a_package_without_a_signed_module() -> TestResult {
    let fixture = ApprovalFixture::install(None).await?;
    fixture.enable_preview().await?;
    assert!(matches!(
        fixture.prepare(fixture.attempt()).await,
        Err(AppError::Forbidden(_))
    ));
    Ok(())
}

#[tokio::test]
#[ignore = "build the standalone mundam-runtime-probe helper first"]
async fn plugin_preview_approval_reaches_supervised_helper_and_fences_revocation() -> TestResult {
    use mundam_runtime_transport::{preview::PreviewRead, supervised::run_with_preview, Outcome};
    use std::{future, io, path::Path};

    let helper = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sdk/plugins/runtime-probe/target/debug")
        .join(format!(
            "mundam-runtime-probe{}",
            std::env::consts::EXE_SUFFIX
        ))
        .canonicalize()?;
    let fixture = ApprovalFixture::new().await?;
    fixture.enable_preview().await?;
    let directory = fixture.preview.fixture.data.path().canonicalize()?;

    let (module, mut preview) = fixture
        .prepare(fixture.attempt())
        .await?
        .admit_launch()
        .await?;
    let offer = preview.offer();
    let outcome = run_with_preview(
        &helper,
        &directory,
        &module,
        u32::try_from(offer.byte_count)?,
        Duration::from_secs(2),
        future::pending(),
        async |request: PreviewRead| {
            preview
                .read(
                    offer.handle,
                    request.offset as usize,
                    request.length as usize,
                )
                .await
                .map(<[u8]>::to_vec)
                .map_err(io::Error::other)
        },
    )
    .await?;
    assert_eq!(outcome, Outcome::Completed(i32::from_le_bytes(*b"RIFF")));

    let (module, mut preview) = fixture
        .prepare(fixture.attempt())
        .await?
        .admit_launch()
        .await?;
    let offer = preview.offer();
    let other_registry =
        Arc::new(SqlitePluginRegistry::open(&registry_path(&fixture.preview)).await?);
    let other_service = PluginService::new(other_registry);
    let mut approved_reads = 0;
    let outcome = run_with_preview(
        &helper,
        &directory,
        &module,
        u32::try_from(offer.byte_count)?,
        Duration::from_secs(2),
        future::pending(),
        async |request: PreviewRead| {
            if request.offset != 0 {
                other_service
                    .revoke_publisher(fixture.publisher.clone())
                    .await
                    .map_err(io::Error::other)?;
            }
            let bytes = preview
                .read(
                    offer.handle,
                    request.offset as usize,
                    request.length as usize,
                )
                .await
                .map_err(|error| match error {
                    AppError::Forbidden(_) => {
                        io::Error::new(io::ErrorKind::PermissionDenied, error)
                    }
                    _ => io::Error::other(error),
                })?;
            approved_reads += 1;
            Ok(bytes.to_vec())
        },
    )
    .await;
    assert_eq!(approved_reads, 1);
    assert!(matches!(outcome, Err(error) if error.kind() == io::ErrorKind::PermissionDenied));
    Ok(())
}

#[tokio::test]
#[ignore = "build the component-proof helper and generate the WIT component first"]
async fn plugin_preview_approval_executes_the_signed_wit_component() -> TestResult {
    use mundam_runtime_transport::{preview::PreviewRead, supervised::run_with_preview, Outcome};
    use std::{future, io, path::Path};

    let sdk = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sdk/plugins");
    let helper = sdk
        .join("runtime-probe/target/debug")
        .join(format!(
            "mundam-runtime-probe{}",
            std::env::consts::EXE_SUFFIX
        ))
        .canonicalize()?;
    let component = tokio::fs::read(sdk.join(
        "v1/examples/wit-preview-check/target/wasm32-unknown-unknown/release/preview-check.component.wasm"
    )).await?;
    let fixture = ApprovalFixture::install(Some(("wit-preview-check-v1", &component))).await?;
    fixture.enable_preview().await?;
    let directory = fixture.preview.fixture.data.path().canonicalize()?;
    let other_registry =
        Arc::new(SqlitePluginRegistry::open(&registry_path(&fixture.preview)).await?);
    let other_service = PluginService::new(other_registry);

    for revoke in [false, true] {
        let (module, mut preview) = fixture
            .prepare(fixture.attempt())
            .await?
            .admit_launch()
            .await?;
        assert_eq!(module, component);
        let offer = preview.offer();
        let mut approved_ranges = Vec::new();
        let outcome = run_with_preview(
            &helper,
            &directory,
            &module,
            u32::try_from(offer.byte_count)?,
            Duration::from_secs(2),
            future::pending(),
            async |request: PreviewRead| {
                if revoke && request.offset == 8 {
                    other_service
                        .revoke_publisher(fixture.publisher.clone())
                        .await
                        .map_err(io::Error::other)?;
                }
                let bytes = preview
                    .read(
                        offer.handle,
                        request.offset as usize,
                        request.length as usize,
                    )
                    .await
                    .map_err(|error| match error {
                        AppError::Forbidden(_) => {
                            io::Error::new(io::ErrorKind::PermissionDenied, error)
                        }
                        _ => io::Error::other(error),
                    })?;
                approved_ranges.push((request.offset, request.length));
                Ok(bytes.to_vec())
            },
        )
        .await;
        if revoke {
            assert!(
                matches!(outcome, Err(error) if error.kind() == io::ErrorKind::PermissionDenied)
            );
            assert_eq!(approved_ranges, [(0, 4)]);
        } else {
            assert_eq!(outcome?, Outcome::ComponentCompleted(*b"RIFFWEBP"));
            assert_eq!(approved_ranges, [(0, 4), (8, 4)]);
        }
    }
    Ok(())
}
