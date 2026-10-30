use super::*;
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};

fn worker(fixture: &ProcessingFixture) -> ColorWorker {
    ColorWorker::new(
        fixture.ledger.clone(),
        fixture.event_bus.clone(),
        fixture.queries.clone(),
        fixture.processing.clone(),
        fixture.data.path().join("thumbnails"),
    )
}

#[tokio::test]
async fn processing_colors_recovers_without_thumbnail_event_delivery() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", Some("asset.png")).await?;
    image::RgbImage::from_pixel(8, 8, image::Rgb([30, 120, 200]))
        .save(fixture.data.path().join("thumbnails/asset.png"))?;
    worker(&fixture)
        .process_pending(&CancellationToken::new())
        .await?;
    let colors: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM asset_colors")
        .fetch_one(&fixture.pool)
        .await?;
    assert!(colors > 0);
    assert!(fixture
        .processing
        .pending(ProcessingKind::Colors, 1)
        .await?
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn processing_colors_preserves_previous_palette_until_reextraction_succeeds() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", Some("asset.png")).await?;
    sqlx::query(
        "INSERT INTO asset_colors
        (asset_id, hex_color, lab_lightness, lab_green_red, lab_blue_yellow, percentage, rank)
        VALUES ('asset', '#FF0000', 50, 0, 0, 1, 0)",
    )
    .execute(&fixture.pool)
    .await?;
    fixture
        .ledger
        .execute(LedgerCommand::ReextractColors {
            asset_id: "asset".into(),
        })
        .await?;
    worker(&fixture)
        .process_pending(&CancellationToken::new())
        .await?;
    let preserved: String =
        sqlx::query_scalar("SELECT hex_color FROM asset_colors WHERE asset_id = 'asset'")
            .fetch_one(&fixture.pool)
            .await?;
    assert_eq!(preserved, "#FF0000");
    let state: String = sqlx::query_scalar(
        "SELECT state FROM processing_tasks WHERE asset_id = 'asset' AND kind = 'colors'",
    )
    .fetch_one(&fixture.pool)
    .await?;
    assert_eq!(state, "retry_wait");
    image::RgbImage::from_pixel(8, 8, image::Rgb([30, 120, 200]))
        .save(fixture.data.path().join("thumbnails/asset.png"))?;
    fixture
        .ledger
        .execute(LedgerCommand::ReextractColors {
            asset_id: "asset".into(),
        })
        .await?;
    worker(&fixture)
        .process_pending(&CancellationToken::new())
        .await?;
    let replacement: String = sqlx::query_scalar(
        "SELECT hex_color FROM asset_colors WHERE asset_id = 'asset' ORDER BY rank LIMIT 1",
    )
    .fetch_one(&fixture.pool)
    .await?;
    assert_ne!(replacement, preserved);
    Ok(())
}

#[tokio::test]
async fn processing_colors_cancellation_leaves_work_recoverable() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", Some("asset.png")).await?;
    let token = CancellationToken::new();
    token.cancel();
    worker(&fixture).process_pending(&token).await?;
    assert_eq!(
        fixture
            .processing
            .pending(ProcessingKind::Colors, 1)
            .await?,
        ["asset"]
    );
    Ok(())
}
