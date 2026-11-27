//! Verifies bounded signed packages without extraction or filesystem access.
//!
//! Each envelope version fixes the order of its resources and signing key.
//! Trust is supplied by the host separately from the package being inspected.

use crate::{
    parse_manifest, validate_icon, ValidatedManifest, ValidationError, WorkerAbi, MAX_ICON_BYTES,
    MAX_MANIFEST_BYTES,
};
use ring::digest::{digest, SHA256};
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use std::fmt;

const DECLARATIVE_MAGIC: &[u8] = b"MUNDAM-PLUGIN\0\x01";
const WORKER_MAGIC: &[u8] = b"MUNDAM-PLUGIN\0\x02";
const DECLARATIVE_HEADER_BYTES: usize = DECLARATIVE_MAGIC.len() + 2 * size_of::<u32>();
const WORKER_HEADER_BYTES: usize = WORKER_MAGIC.len() + 3 * size_of::<u32>();
const CORE_WASM_HEADER: &[u8; 8] = b"\0asm\x01\0\0\0";
const COMPONENT_HEADER: &[u8; 8] = b"\0asm\x0d\0\x01\0";
const KEY_BYTES: usize = 32;
const SIGNATURE_BYTES: usize = 64;

/// Shared size budget for both envelope versions, including signature and framing.
pub const MAX_PACKAGE_BYTES: usize =
    DECLARATIVE_HEADER_BYTES + MAX_MANIFEST_BYTES + MAX_ICON_BYTES + KEY_BYTES + SIGNATURE_BYTES;

/// Maximum size of the Wasm payload carried by a worker envelope.
pub const MAX_WORKER_MODULE_BYTES: usize = 256 * 1024;

/// Trust evidence obtained independently of a package, scoped to one exact publisher.
pub struct TrustedPublisher<'a> {
    pub namespace: &'a str,
    pub key_fingerprint: [u8; 32],
}

/// Verified package information bound to the bytes available for admission.
pub struct VerifiedPackage<'a> {
    manifest: ValidatedManifest,
    icon: &'a [u8],
    worker_module: Option<&'a [u8]>,
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

    /// Returns the exact signed Wasm payload after checking its envelope and declaration.
    pub fn worker_module(&self) -> Option<&'a [u8]> {
        self.worker_module
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
            Self::InvalidEnvelope => formatter.write_str("invalid plugin package envelope"),
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
/// Rejects invalid manifest or icon content, worker declarations and invalid signing keys.
pub fn sign_package(manifest: &[u8], icon: &[u8], pkcs8: &[u8]) -> Result<Vec<u8>, PackageError> {
    sign(manifest, icon, None, pkcs8)
}

/// Builds the v2 envelope with one declared worker and its exact signed Wasm payload.
///
/// # Errors
/// Rejects invalid resources or key, mismatched worker declarations and modules,
/// and packages larger than the shared size budget.
pub fn sign_worker_package(
    manifest: &[u8],
    icon: &[u8],
    module: &[u8],
    pkcs8: &[u8],
) -> Result<Vec<u8>, PackageError> {
    sign(manifest, icon, Some(module), pkcs8)
}

fn sign(
    manifest: &[u8],
    icon: &[u8],
    module: Option<&[u8]>,
    pkcs8: &[u8],
) -> Result<Vec<u8>, PackageError> {
    let declaration = parse_manifest(manifest).map_err(PackageError::InvalidContent)?;
    validate_resources(&declaration, manifest.len(), icon, module)?;
    let key = signing_key(pkcs8)?;
    let mut bytes = Vec::with_capacity(package_size(manifest.len(), icon.len(), module));
    let magic = if module.is_some() {
        WORKER_MAGIC
    } else {
        DECLARATIVE_MAGIC
    };
    bytes.extend_from_slice(magic);
    bytes.extend_from_slice(&(manifest.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&(icon.len() as u32).to_be_bytes());
    if let Some(module) = module {
        bytes.extend_from_slice(&(module.len() as u32).to_be_bytes());
    }
    bytes.extend_from_slice(manifest);
    bytes.extend_from_slice(icon);
    if let Some(module) = module {
        bytes.extend_from_slice(module);
    }
    bytes.extend_from_slice(key.public_key().as_ref());
    let signature = key.sign(&bytes);
    bytes.extend_from_slice(signature.as_ref());
    Ok(bytes)
}

fn package_size(manifest_len: usize, icon_len: usize, module: Option<&[u8]>) -> usize {
    let header = if module.is_some() {
        WORKER_HEADER_BYTES
    } else {
        DECLARATIVE_HEADER_BYTES
    };
    header + manifest_len + icon_len + module.map_or(0, <[u8]>::len) + KEY_BYTES + SIGNATURE_BYTES
}

/// Checks package resources without parsing guest instructions in the trusted host.
///
/// # Errors
/// Rejects an invalid icon, a missing or undeclared payload, a mismatched Wasm header,
/// and resources that would exceed the package budget.
pub fn validate_resources(
    declaration: &ValidatedManifest,
    manifest_len: usize,
    icon: &[u8],
    module: Option<&[u8]>,
) -> Result<(), PackageError> {
    let worker = declaration.manifest().worker.as_ref();
    if worker.is_some() != module.is_some() {
        return Err(PackageError::InvalidContent(ValidationError::new(
            "worker",
            "a worker declaration and its module must be supplied together",
        )));
    }
    if let (Some(worker), Some(bytes)) = (worker, module) {
        let header = match worker.abi {
            WorkerAbi::PreviewProbeV1 => CORE_WASM_HEADER,
            WorkerAbi::WitPreviewCheckV1 => COMPONENT_HEADER,
        };
        if bytes.len() > MAX_WORKER_MODULE_BYTES || !bytes.starts_with(header) {
            return Err(PackageError::InvalidContent(ValidationError::new(
                "worker",
                "expected a Wasm payload matching the declared ABI and at most 256 KiB",
            )));
        }
    }
    validate_icon(icon).map_err(PackageError::InvalidContent)?;
    if !(1..=MAX_MANIFEST_BYTES).contains(&manifest_len)
        || package_size(manifest_len, icon.len(), module) > MAX_PACKAGE_BYTES
    {
        return Err(PackageError::InvalidEnvelope);
    }
    Ok(())
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
    validate_resources(
        &manifest,
        envelope.manifest.len(),
        envelope.icon,
        envelope.worker_module,
    )?;
    Ok(VerifiedPackage {
        manifest,
        icon: envelope.icon,
        worker_module: envelope.worker_module,
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
    worker_module: Option<&'a [u8]>,
    public_key: &'a [u8],
    signature: &'a [u8],
}

impl<'a> Envelope<'a> {
    fn parse(bytes: &'a [u8]) -> Option<Self> {
        if bytes.len() > MAX_PACKAGE_BYTES {
            return None;
        }
        let (header_end, has_module) = if bytes.starts_with(WORKER_MAGIC) {
            (WORKER_HEADER_BYTES, true)
        } else if bytes.starts_with(DECLARATIVE_MAGIC) {
            (DECLARATIVE_HEADER_BYTES, false)
        } else {
            return None;
        };
        let mut lengths = bytes
            .get(DECLARATIVE_MAGIC.len()..header_end)?
            .chunks_exact(4);
        let mut next_length =
            || Some(u32::from_be_bytes(lengths.next()?.try_into().ok()?) as usize);
        let manifest_size = next_length()?;
        let icon_size = next_length()?;
        let module_size = if has_module { next_length()? } else { 0 };
        if !(1..=MAX_MANIFEST_BYTES).contains(&manifest_size)
            || !(1..=MAX_ICON_BYTES).contains(&icon_size)
            || (has_module
                && !(CORE_WASM_HEADER.len()..=MAX_WORKER_MODULE_BYTES).contains(&module_size))
        {
            return None;
        }
        let manifest_end = header_end + manifest_size;
        let icon_end = manifest_end + icon_size;
        let module_end = icon_end + module_size;
        let signed_end = module_end + KEY_BYTES;
        if bytes.len() != signed_end + SIGNATURE_BYTES {
            return None;
        }
        Some(Self {
            signed_bytes: &bytes[..signed_end],
            manifest: &bytes[header_end..manifest_end],
            icon: &bytes[manifest_end..icon_end],
            worker_module: has_module.then_some(&bytes[icon_end..module_end]),
            public_key: &bytes[module_end..signed_end],
            signature: &bytes[signed_end..],
        })
    }
}
