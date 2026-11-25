use super::*;
use serde_json::{json, Value};

const EXAMPLE: &str = include_str!("../../../../docs/api/v1/examples/import.json");

fn limits() -> ImportLimits {
    ImportLimits {
        max_items: 2,
        max_tags_per_item: 2,
        max_item_bytes: 78_041,
        max_batch_bytes: 96_073,
    }
}

#[test]
fn published_manifest_matches_negotiated_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let manifest: ImportManifest = serde_json::from_str(EXAMPLE)?;
    let normalized = manifest.normalize(&limits())?;
    assert_eq!(normalized.folder_id.as_deref(), Some("folder-referencias"));
    assert!(normalized.tag_ids.is_empty());
    assert_eq!(normalized.items[0].tag_ids, ["tag-pesquisa"]);
    assert_eq!(normalized.items[1].tag_ids, ["tag-mapas", "tag-pesquisa"]);
    Ok(())
}

#[test]
fn normalization_preserves_default_intent_and_is_idempotent(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut manifest: ImportManifest = serde_json::from_str(EXAMPLE)?;
    manifest.folder_id = None;
    manifest.items[0].source_url = Some(" https://example.org/page?q=a#b ".into());
    manifest.items[1].tag_ids.push("tag-pesquisa".into());
    let result = manifest.normalize(&limits())?;
    assert!(result.folder_id.is_none());
    assert_eq!(
        result.items[0].source_url.as_deref(),
        Some("https://example.org/page?q=a#b")
    );
    let value = serde_json::to_value(&result)?;
    assert_eq!(value, serde_json::to_value(result.normalize(&limits())?)?);
    Ok(())
}

#[test]
fn invalid_inputs_fail_before_staging() -> Result<(), Box<dyn std::error::Error>> {
    for (field, value) in [
        ("itemId", json!("capa")),
        ("itemId", json!("a/b")),
        ("fileName", json!("../mapa.png")),
        ("fileName", json!("NUL.png")),
        ("sourceUrl", json!("file:///mapa.png")),
        ("tagIds", json!(["tag-2", "tag-3"])),
        ("tagIds", json!([" "])),
        ("byteLength", json!(78_042)),
    ] {
        let mut input: Value = serde_json::from_str(EXAMPLE)?;
        input["items"][1][field] = value;
        let manifest: ImportManifest = serde_json::from_value(input)?;
        assert!(manifest.normalize(&limits()).is_err(), "{field}");
    }
    let mut manifest: ImportManifest = serde_json::from_str(EXAMPLE)?;
    manifest.items[0].byte_length += 1;
    assert!(manifest.normalize(&limits()).is_err());
    Ok(())
}
