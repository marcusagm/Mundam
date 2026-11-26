//! Verifies a bounded declarative package without extraction or filesystem access.
//!
//! The v1 header identifies the signing domain and fixes the order of the
//! manifest, icon and public key. Each file is covered byte-for-byte by Ed25519.
//! Trust is supplied by the host separately from the package being inspected.

use crate::{
    parse_manifest, validate_icon, ValidatedManifest, ValidationError, MAX_ICON_BYTES,
    MAX_MANIFEST_BYTES,
};
use ring::digest::{digest, SHA256};
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use std::fmt;

const MAGIC: &[u8] = b"MUNDAM-PLUGIN\0\x01";
const LENGTH_BYTES: usize = 2 * size_of::<u32>();
const HEADER_BYTES: usize = MAGIC.len() + LENGTH_BYTES;
const KEY_BYTES: usize = 32;
const SIGNATURE_BYTES: usize = 64;

/// Maximum size of a v1 package, including its signature and framing.
pub const MAX_PACKAGE_BYTES: usize =
    HEADER_BYTES + MAX_MANIFEST_BYTES + MAX_ICON_BYTES + KEY_BYTES + SIGNATURE_BYTES;

/// Trust evidence obtained independently of a package, scoped to one exact publisher.
pub struct TrustedPublisher<'a> {
    pub namespace: &'a str,
    pub key_fingerprint: [u8; 32],
}

/// Verified package information bound to the bytes available for admission.
pub struct VerifiedPackage<'a> {
    manifest: ValidatedManifest,
    icon: &'a [u8],
    package_digest: [u8; 32],
}

impl<'a> VerifiedPackage<'a> {
    /// Returns the declaration checked against the trusted publisher namespace.
    pub fn manifest(&self) -> &ValidatedManifest {
        &self.manifest
    }

    /// Returns the signed, decoded successfully as PNG, icon bytes.
    pub fn icon(&self) -> &'a [u8] {
        self.icon
    }

    /// Identifies the exact accepted bytes for binding a future install approval.
    pub fn package_digest(&self) -> [u8; 32] {
        self.package_digest
    }
}

/// Reason why authoring or admission failed before exposing package content.
#[derive(Debug)]
pub enum PackageError {
    InvalidEnvelope,
    InvalidSigningKey,
    UntrustedKey,
    InvalidSignature,
    PublisherMismatch,
    InvalidContent(ValidationError),
}

impl fmt::Display for PackageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEnvelope => formatter.write_str("invalid v1 plugin package envelope"),
            Self::InvalidSigningKey => formatter.write_str("invalid Ed25519 PKCS#8 signing key"),
            Self::UntrustedKey => formatter.write_str("package key does not match the trusted key"),
            Self::InvalidSignature => formatter.write_str("plugin package signature is invalid"),
            Self::PublisherMismatch => {
                formatter.write_str("package publisher is outside the trusted namespace")
            }
            Self::InvalidContent(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PackageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidContent(error) => Some(error),
            _ => None,
        }
    }
}

/// Computes the publisher key fingerprint from a local PKCS#8 DER signing key.
///
/// # Errors
/// Rejects a key that cannot be parsed as Ed25519 PKCS#8.
pub fn signing_key_fingerprint(pkcs8: &[u8]) -> Result<[u8; 32], PackageError> {
    let key = signing_key(pkcs8)?;
    Ok(sha256(key.public_key().as_ref()))
}

/// Returns an unverified key hint for looking up a separately trusted publisher.
///
/// # Errors
/// Rejects invalid envelope lengths and unsupported formats.
pub fn package_key_hint(bytes: &[u8]) -> Result<[u8; 32], PackageError> {
    let envelope = Envelope::parse(bytes).ok_or(PackageError::InvalidEnvelope)?;
    Ok(sha256(envelope.public_key))
}

/// Builds the v1 envelope from the exact manifest and PNG supplied by the author.
///
/// # Errors
/// Rejects invalid manifest or icon content and invalid Ed25519 signing keys.
pub fn sign_package(manifest: &[u8], icon: &[u8], pkcs8: &[u8]) -> Result<Vec<u8>, PackageError> {
    parse_manifest(manifest).map_err(PackageError::InvalidContent)?;
    validate_icon(icon).map_err(PackageError::InvalidContent)?;
    let key = signing_key(pkcs8)?;
    let mut bytes = Vec::with_capacity(
        HEADER_BYTES + manifest.len() + icon.len() + KEY_BYTES + SIGNATURE_BYTES,
    );
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&(manifest.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&(icon.len() as u32).to_be_bytes());
    bytes.extend_from_slice(manifest);
    bytes.extend_from_slice(icon);
    bytes.extend_from_slice(key.public_key().as_ref());
    let signature = key.sign(&bytes);
    bytes.extend_from_slice(signature.as_ref());
    Ok(bytes)
}

/// Verifies completeness, authorized signer and content before a host can display it.
///
/// # Errors
/// Rejects malformed packages, untrusted keys, invalid signatures, mismatched
/// publishers and invalid content. Target compatibility is evaluated separately
/// by the caller against its actual executor and supported API version.
pub fn verify_package<'a>(
    bytes: &'a [u8],
    trust: &TrustedPublisher<'_>,
) -> Result<VerifiedPackage<'a>, PackageError> {
    let envelope = Envelope::parse(bytes).ok_or(PackageError::InvalidEnvelope)?;
    if sha256(envelope.public_key) != trust.key_fingerprint {
        return Err(PackageError::UntrustedKey);
    }
    UnparsedPublicKey::new(&ED25519, envelope.public_key)
        .verify(envelope.signed_bytes, envelope.signature)
        .map_err(|_| PackageError::InvalidSignature)?;
    let manifest = parse_manifest(envelope.manifest).map_err(PackageError::InvalidContent)?;
    if manifest.manifest().publisher != trust.namespace {
        return Err(PackageError::PublisherMismatch);
    }
    validate_icon(envelope.icon).map_err(PackageError::InvalidContent)?;
    Ok(VerifiedPackage {
        manifest,
        icon: envelope.icon,
        package_digest: sha256(bytes),
    })
}

fn signing_key(pkcs8: &[u8]) -> Result<Ed25519KeyPair, PackageError> {
    // OpenSSL also emits PKCS#8 v1 without an embedded public key. Ring derives it
    // from the private seed and still checks consistency when both are present.
    Ed25519KeyPair::from_pkcs8_maybe_unchecked(pkcs8).map_err(|_| PackageError::InvalidSigningKey)
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hash = [0; 32];
    hash.copy_from_slice(digest(&SHA256, bytes).as_ref());
    hash
}

struct Envelope<'a> {
    signed_bytes: &'a [u8],
    manifest: &'a [u8],
    icon: &'a [u8],
    public_key: &'a [u8],
    signature: &'a [u8],
}

impl<'a> Envelope<'a> {
    fn parse(bytes: &'a [u8]) -> Option<Self> {
        if bytes.len() > MAX_PACKAGE_BYTES || !bytes.starts_with(MAGIC) {
            return None;
        }
        let header: &[u8; LENGTH_BYTES] = bytes.get(MAGIC.len()..HEADER_BYTES)?.try_into().ok()?;
        let manifest_size = u32::from_be_bytes(header[..4].try_into().ok()?) as usize;
        let icon_size = u32::from_be_bytes(header[4..].try_into().ok()?) as usize;
        if !(1..=MAX_MANIFEST_BYTES).contains(&manifest_size)
            || !(1..=MAX_ICON_BYTES).contains(&icon_size)
        {
            return None;
        }
        let manifest_end = HEADER_BYTES + manifest_size;
        let icon_end = manifest_end + icon_size;
        let signed_end = icon_end + KEY_BYTES;
        if bytes.len() != signed_end + SIGNATURE_BYTES {
            return None;
        }
        Some(Self {
            signed_bytes: &bytes[..signed_end],
            manifest: &bytes[HEADER_BYTES..manifest_end],
            icon: &bytes[manifest_end..icon_end],
            public_key: &bytes[icon_end..signed_end],
            signature: &bytes[signed_end..],
        })
    }
}
