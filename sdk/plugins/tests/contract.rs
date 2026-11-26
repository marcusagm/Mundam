//! Treats the v1 examples as external inputs, using the public SDK and author CLI.

use mundam_plugin_sdk::{parse_manifest, validate_icon, Executor, Version, MAX_MANIFEST_BYTES};
use serde_json::{json, Value};
use std::error::Error;
use std::path::Path;
use std::process::Command;

const UNIVERSAL: &[u8] = include_bytes!("../v1/examples/reference-review/manifest.json");
const SPECIFIC: &[u8] = include_bytes!("../v1/examples/windows-verification/manifest.json");
const REVIEW_ICON: &[u8] = include_bytes!("../v1/examples/reference-review/assets/review.png");
const WINDOWS_ICON: &[u8] =
    include_bytes!("../v1/examples/windows-verification/assets/verification.png");

#[test]
fn screens_can_be_declared_without_asset_permissions() -> Result<(), Box<dyn Error>> {
    let mut declaration: Value = serde_json::from_slice(UNIVERSAL)?;
    declaration["permissions"] = json!([]);
    declaration["contributes"] = json!({"screens": declaration["contributes"]["screens"]});
    let validated = parse_manifest(&serde_json::to_vec(&declaration)?)?;
    assert_eq!(
        validated.manifest().contributes.screens[0].id,
        "review-guide"
    );
    let screen = declaration["contributes"]["screens"][0].clone();
    declaration["contributes"]["screens"] = json!([screen, screen]);
    assert_eq!(
        parse_manifest(&serde_json::to_vec(&declaration)?)
            .expect_err("duplicate screen")
            .field,
        "contributes.screens"
    );
    declaration["contributes"]["screens"] = json!([screen]);
    declaration["contributes"]["screens"][0]["blocks"][0] =
        json!({"kind": "html", "text": "markup"});
    assert!(parse_manifest(&serde_json::to_vec(&declaration)?).is_err());
    Ok(())
}

#[test]
fn platform_pairs_and_stable_api_bounds_are_evaluated_together() -> Result<(), Box<dyn Error>> {
    let mut declarations: Value = serde_json::from_slice(SPECIFIC)?;
    declarations["targets"] = json!([
        {"platform": "windows", "architecture": "x64"},
        {"platform": "mac", "architecture": "arm64"},
        {"platform": "linux", "architecture": "all"}
    ]);
    let packages = [
        parse_manifest(UNIVERSAL)?,
        parse_manifest(&serde_json::to_vec(&declarations)?)?,
    ];
    let api = Version::parse("1.0.0+build.7")?;
    for (platform, architecture, expected) in [
        ("windows", "x86_64", true),
        ("windows", "aarch64", false),
        ("macos", "aarch64", true),
        ("macos", "x86_64", false),
        ("linux", "arm", true),
    ] {
        let executor =
            Executor::from_system(platform, architecture).ok_or("unknown test target")?;
        assert!(packages[0].supports(executor, &api));
        assert_eq!(packages[1].supports(executor, &api), expected);
        for unavailable in ["0.9.9", "1.5.0-beta.1", "2.0.0"] {
            assert!(!packages[0].supports(executor, &Version::parse(unavailable)?));
        }
    }
    assert!(Executor::from_system("freebsd", "x86_64").is_none());
    assert!(Executor::from_system("linux", "riscv64").is_none());
    Ok(())
}

#[test]
fn normalizes_search_terms_without_changing_the_first_display_spelling(
) -> Result<(), Box<dyn Error>> {
    let mut declaration: Value = serde_json::from_slice(UNIVERSAL)?;
    declaration["keywords"] = json!(["REVISÃO", "revisa\u{0303}o", "review"]);
    let validated = parse_manifest(&serde_json::to_vec(&declaration)?)?;
    assert_eq!(validated.manifest().keywords, ["REVISÃO", "review"]);
    Ok(())
}

#[test]
fn rejects_invalid_declarations_before_they_reach_a_renderer() -> Result<(), Box<dyn Error>> {
    let original: Value = serde_json::from_slice(UNIVERSAL)?;
    for (pointer, invalid, field) in [
        ("/manifestVersion", json!(2), "manifestVersion"),
        ("/id", json!("org.other.review"), "id"),
        ("/version", json!("01.0.0"), "version"),
        ("/icon", json!("assets/../review.png"), "icon"),
        ("/icon", json!("assets/con.png"), "icon"),
        ("/description", json!(""), "description"),
        (
            "/engines/mundamApi",
            json!(">=2.0.0 <1.0.0"),
            "engines.mundamApi",
        ),
        (
            "/targets",
            json!([original["targets"][0], original["targets"][0]]),
            "targets",
        ),
        ("/targets/0/platform", json!("macos"), "manifest"),
        ("/targets/0/architecture", json!("x86"), "manifest"),
        ("/permissions/2", json!("shell.run"), "manifest"),
        ("/permissions/2", json!("assetData.own.read"), "permissions"),
        (
            "/permissions",
            json!(["selection.read", "assetData.own.read"]),
            "permissions",
        ),
        (
            "/contributes/assetFields/0/schemaVersion",
            json!(0),
            "contributes.assetFields",
        ),
        (
            "/contributes/inspectorSections/0/fields/0/field",
            json!("missing"),
            "contributes.inspectorSections",
        ),
        (
            "/contributes/inspectorSections/0/when/selectionCount",
            json!(2),
            "contributes.inspectorSections",
        ),
        (
            "/contributes/inspectorSections/0/viewportActionLabel",
            json!("Show review\n"),
            "contributes.inspectorSections.viewportActionLabel",
        ),
        (
            "/contributes/inspectorSections/0/sidebarLinkLabel",
            json!("a".repeat(81)),
            "contributes.inspectorSections.sidebarLinkLabel",
        ),
        (
            "/contributes/inspectorSections/0/titleBarActionLabel",
            json!("Show\tReview"),
            "contributes.inspectorSections.titleBarActionLabel",
        ),
    ] {
        let mut declaration = original.clone();
        *declaration
            .pointer_mut(pointer)
            .ok_or("bad fixture pointer")? = invalid;
        let diagnostic = parse_manifest(&serde_json::to_vec(&declaration)?).expect_err(pointer);
        assert_eq!(diagnostic.field, field, "{pointer}: {diagnostic}");
    }
    let mut declaration = original.clone();
    declaration["optionalPermissions"] = json!(["selection.read"]);
    assert!(parse_manifest(&serde_json::to_vec(&declaration)?).is_err());
    declaration["contributes"]["script"] = json!("main.js");
    assert!(parse_manifest(&serde_json::to_vec(&declaration)?).is_err());
    let duplicate = String::from_utf8(UNIVERSAL.to_vec())?.replacen(
        "\"manifestVersion\": 1",
        "\"manifestVersion\": 1, \"manifestVersion\": 1",
        1,
    );
    assert!(parse_manifest(duplicate.as_bytes()).is_err());
    assert!(parse_manifest(&vec![b' '; MAX_MANIFEST_BYTES + 1]).is_err());
    Ok(())
}

#[test]
fn verifies_png_contents_and_bounds() -> Result<(), Box<dyn Error>> {
    validate_icon(REVIEW_ICON)?;
    validate_icon(WINDOWS_ICON)?;
    assert!(validate_icon(&REVIEW_ICON[..REVIEW_ICON.len() - 12]).is_err());
    assert!(validate_icon(b"<svg/>").is_err());
    assert!(validate_icon(&vec![0; 256 * 1024 + 1]).is_err());
    for (width, animated) in [(15, false), (257, false), (32, true)] {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, 32);
        if animated {
            encoder.set_animated(1, 0)?;
        }
        encoder
            .write_header()?
            .write_image_data(&vec![0; (width * 32) as usize])?;
        assert!(validate_icon(&bytes).is_err());
    }
    Ok(())
}

#[test]
fn cli_output_can_drive_an_independent_inspector_consumer() -> Result<(), Box<dyn Error>> {
    let binary = env!("CARGO_BIN_EXE_mundam-plugin");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("v1/examples");
    for (package, expected_id, expected_field, read_only) in [
        (
            "reference-review",
            "org.example.reference-review",
            "reviewed",
            false,
        ),
        (
            "windows-verification",
            "org.example.windows-verification",
            "verified",
            true,
        ),
    ] {
        let output = Command::new(binary)
            .arg("validate")
            .arg(fixtures.join(package).join("manifest.json"))
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let projection: Value = serde_json::from_slice(&output.stdout)?;
        let manifest = &projection["manifest"];
        let binding = &manifest["contributes"]["inspectorSections"][0]["fields"][0];
        let fields = manifest["contributes"]["assetFields"]
            .as_array()
            .ok_or("missing fields")?;
        let field = fields
            .iter()
            .find(|field| field["id"] == binding["field"])
            .ok_or("unbound field")?;
        assert_eq!(manifest["id"], expected_id);
        assert_eq!(field["id"], expected_field);
        assert_eq!(field["default"], false);
        assert_eq!(binding["readOnly"], read_only);
    }
    let schema = Command::new(binary).arg("schema").output()?;
    assert!(schema.status.success());
    assert_eq!(schema.stdout, mundam_plugin_sdk::MANIFEST_SCHEMA.as_bytes());
    let unsupported = Command::new(binary)
        .arg("validate")
        .arg(fixtures.join("reference-review/manifest.json"))
        .args(["--api-version", "2.0.0"])
        .output()?;
    assert!(unsupported.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&unsupported.stdout)?["compatible"],
        false
    );
    Ok(())
}
