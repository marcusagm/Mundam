//! Exercises the signed distribution boundary through the library and authoring CLI.

use mundam_plugin_sdk::package::{
    sign_package, signing_key_fingerprint, verify_package, PackageError, TrustedPublisher,
    MAX_PACKAGE_BYTES,
};
use ring::digest::{digest, SHA256};
use ring::rand::SystemRandom;
use ring::signature::Ed25519KeyPair;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

const MANIFEST: &[u8] = include_bytes!("../v1/examples/reference-review/manifest.json");
const ICON: &[u8] = include_bytes!("../v1/examples/reference-review/assets/review.png");

fn generate_key() -> Result<ring::pkcs8::Document, &'static str> {
    Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
        .map_err(|_| "system randomness was unavailable for the test key")
}

#[test]
fn signed_content_is_bound_to_exact_publisher_key_and_package_bytes() -> Result<(), Box<dyn Error>>
{
    let key = generate_key()?;
    let fingerprint = signing_key_fingerprint(key.as_ref())?;
    let package = sign_package(MANIFEST, ICON, key.as_ref())?;
    let verified = verify_package(
        &package,
        &TrustedPublisher {
            namespace: "org.example",
            key_fingerprint: fingerprint,
        },
    )?;
    assert_eq!(
        verified.manifest().manifest().id,
        "org.example.reference-review"
    );
    assert_eq!(verified.icon(), ICON);
    assert_eq!(
        verified.package_digest(),
        digest(&SHA256, &package).as_ref()
    );

    let unrelated_key = generate_key()?;
    let wrong_key = TrustedPublisher {
        namespace: "org.example",
        key_fingerprint: signing_key_fingerprint(unrelated_key.as_ref())?,
    };
    assert!(matches!(
        verify_package(&package, &wrong_key),
        Err(PackageError::UntrustedKey)
    ));

    let wrong_namespace = TrustedPublisher {
        namespace: "org.example.other",
        key_fingerprint: fingerprint,
    };
    assert!(matches!(
        verify_package(&package, &wrong_namespace),
        Err(PackageError::PublisherMismatch)
    ));
    Ok(())
}

#[test]
fn refuses_changed_signed_files_and_malformed_framing() -> Result<(), Box<dyn Error>> {
    let key = generate_key()?;
    let package = sign_package(MANIFEST, ICON, key.as_ref())?;
    let trust = TrustedPublisher {
        namespace: "org.example",
        key_fingerprint: signing_key_fingerprint(key.as_ref())?,
    };

    let first_manifest_byte = package
        .windows(MANIFEST.len())
        .position(|part| part == MANIFEST)
        .ok_or("missing manifest bytes")?;
    let first_icon_byte = first_manifest_byte + MANIFEST.len();
    for offset in [first_manifest_byte, first_icon_byte, package.len() - 1] {
        let mut changed = package.clone();
        changed[offset] ^= 1;
        assert!(matches!(
            verify_package(&changed, &trust),
            Err(PackageError::InvalidSignature)
        ));
    }

    for length in 0..package.len() {
        assert!(matches!(
            verify_package(&package[..length], &trust),
            Err(PackageError::InvalidEnvelope)
        ));
    }
    let mut extra = package.clone();
    extra.push(0);
    assert!(matches!(
        verify_package(&extra, &trust),
        Err(PackageError::InvalidEnvelope)
    ));

    let mut inconsistent = package.clone();
    inconsistent[first_manifest_byte - 8..first_manifest_byte - 4].fill(0xff);
    assert!(matches!(
        verify_package(&inconsistent, &trust),
        Err(PackageError::InvalidEnvelope)
    ));
    let oversized = vec![0; MAX_PACKAGE_BYTES + 1];
    assert!(matches!(
        verify_package(&oversized, &trust),
        Err(PackageError::InvalidEnvelope)
    ));
    assert!(matches!(
        sign_package(MANIFEST, ICON, b"invalid"),
        Err(PackageError::InvalidSigningKey)
    ));
    assert!(matches!(
        sign_package(MANIFEST, b"invalid", key.as_ref()),
        Err(PackageError::InvalidContent(_))
    ));
    Ok(())
}

#[test]
fn cli_packs_and_verifies_a_file_with_an_explicit_trust_anchor() -> Result<(), Box<dyn Error>> {
    let binary = env!("CARGO_BIN_EXE_mundam-plugin");
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("v1/examples/reference-review/manifest.json");
    let directory = tempfile::tempdir()?;
    let key_path = directory.path().join("test-key.der");
    let package_path = directory.path().join("review.mundam-plugin");
    fs::write(&key_path, generate_key()?.as_ref())?;

    let fingerprint = Command::new(binary)
        .arg("key-fingerprint")
        .arg(&key_path)
        .output()?;
    assert!(
        fingerprint.status.success(),
        "{}",
        String::from_utf8_lossy(&fingerprint.stderr)
    );
    let fingerprint = std::str::from_utf8(&fingerprint.stdout)?.trim();
    assert_eq!(fingerprint.len(), 64);

    let mut pack = Command::new(binary);
    pack.arg("pack")
        .arg(fixture)
        .arg(&key_path)
        .arg(&package_path);
    let result = pack.output()?;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let package = fs::read(&package_path)?;
    assert!(!pack.output()?.status.success());
    assert_eq!(fs::read(&package_path)?, package);

    let mut verify = Command::new(binary);
    verify.arg("verify").arg(&package_path);
    let report = verify
        .args(["org.example", fingerprint, "--api-version", "2.0.0"])
        .output()?;
    assert!(
        report.status.success(),
        "{}",
        String::from_utf8_lossy(&report.stderr)
    );
    let report: Value = serde_json::from_slice(&report.stdout)?;
    assert_eq!(report["compatible"], false);
    assert_eq!(report["publisherKeySha256"], fingerprint);
    assert_eq!(report["manifest"]["publisher"], "org.example");
    assert_eq!(report["packageSha256"].as_str().map(str::len), Some(64));

    let rejection = Command::new(binary)
        .arg("verify")
        .arg(&package_path)
        .args(["org.example", &"0".repeat(64)])
        .output()?;
    assert!(!rejection.status.success());
    assert!(rejection.stdout.is_empty());
    Ok(())
}
