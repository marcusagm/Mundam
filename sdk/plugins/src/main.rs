//! Filesystem and output adapter for standalone plugin authoring and verification.

use mundam_plugin_sdk::package::{
    sign_package, sign_worker_package, signing_key_fingerprint, validate_resources, verify_package,
    TrustedPublisher, MAX_PACKAGE_BYTES, MAX_WORKER_MODULE_BYTES,
};
use mundam_plugin_sdk::{
    parse_manifest, Executor, ValidatedManifest, Version, MANIFEST_SCHEMA, MAX_ICON_BYTES,
    MAX_MANIFEST_BYTES,
};
use serde_json::{json, Value};
use std::error::Error;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "Usage: mundam-plugin validate <manifest.json> [--api-version <semver>]\n       mundam-plugin schema\n       mundam-plugin key-fingerprint <ed25519-private.der>\n       mundam-plugin pack <manifest.json> <ed25519-private.der> <output.mundam-plugin>\n       mundam-plugin verify <package.mundam-plugin> <trusted-publisher> <trusted-fingerprint> [--api-version <semver>]";
const MAX_KEY_FILE_BYTES: usize = 1024;

fn read_limited(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    if !fs::metadata(path)?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected a regular file",
        ));
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn icon_path(manifest_path: &Path, relative_path: &str) -> io::Result<std::path::PathBuf> {
    let directory = manifest_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()?;
    let path = directory.join(relative_path);
    let assets = fs::symlink_metadata(directory.join("assets"))?;
    let resource = fs::symlink_metadata(&path)?;
    let canonical_path = path.canonicalize()?;
    if !assets.is_dir() || !resource.is_file() || !canonical_path.starts_with(&directory) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "icon must be a regular file within the package's assets directory",
        ));
    }
    Ok(canonical_path)
}

fn read_icon(
    manifest_path: &Path,
    validated: &ValidatedManifest,
) -> Result<Vec<u8>, Box<dyn Error>> {
    icon_path(manifest_path, &validated.manifest().icon)
        .and_then(|path| read_limited(&path, MAX_ICON_BYTES))
        .map_err(|error| format!("icon: {error}").into())
}

fn read_worker_module(
    manifest_path: &Path,
    validated: &ValidatedManifest,
) -> io::Result<Option<Vec<u8>>> {
    if validated.manifest().worker.is_none() {
        return Ok(None);
    }
    let path = manifest_path.with_file_name("worker.wasm");
    if !fs::symlink_metadata(&path)?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "worker.wasm must be a regular file",
        ));
    }
    read_limited(&path, MAX_WORKER_MODULE_BYTES).map(Some)
}

fn api_version(flags: &[OsString]) -> Result<Version, Box<dyn Error>> {
    let version = match flags {
        [] => "1.0.0",
        [flag, value] if flag == "--api-version" => value.to_str().ok_or(USAGE)?,
        _ => return Err(USAGE.into()),
    };
    Ok(Version::parse(version)?)
}

fn is_compatible(manifest: &ValidatedManifest, version: &Version) -> bool {
    Executor::from_system(std::env::consts::OS, std::env::consts::ARCH)
        .is_some_and(|executor| manifest.supports(executor, version))
}

fn to_hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn fingerprint_argument(value: &std::ffi::OsStr) -> Result<[u8; 32], Box<dyn Error>> {
    let value = value
        .to_str()
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or("expected 64 hexadecimal characters for the trusted key fingerprint")?;
    let mut fingerprint = [0; 32];
    for (position, byte) in fingerprint.iter_mut().enumerate() {
        let start = position * 2;
        *byte = u8::from_str_radix(&value[start..start + 2], 16)?;
    }
    Ok(fingerprint)
}

fn write_json(output: &mut impl Write, value: &Value) -> io::Result<()> {
    serde_json::to_writer_pretty(&mut *output, value)?;
    writeln!(output)
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let action = arguments.first().and_then(|value| value.to_str());
    let mut output = io::stdout().lock();
    match (action, arguments.as_slice()) {
        (Some("schema"), [_]) => output.write_all(MANIFEST_SCHEMA.as_bytes())?,
        (Some("--help" | "-h"), [_]) => writeln!(output, "{USAGE}")?,
        (Some("validate"), [_, manifest_path, flags @ ..]) => {
            let api = api_version(flags)?;
            let manifest_path = Path::new(manifest_path);
            let manifest = read_limited(manifest_path, MAX_MANIFEST_BYTES)?;
            let validated = parse_manifest(&manifest)?;
            let icon = read_icon(manifest_path, &validated)?;
            let module = read_worker_module(manifest_path, &validated)?;
            validate_resources(&validated, manifest.len(), &icon, module.as_deref())?;
            write_json(
                &mut output,
                &json!({
                    "compatible": is_compatible(&validated, &api),
                    "manifest": validated.manifest(),
                }),
            )?;
        }
        (Some("key-fingerprint"), [_, key_path]) => {
            let key = read_limited(Path::new(key_path), MAX_KEY_FILE_BYTES)?;
            writeln!(output, "{}", to_hex(signing_key_fingerprint(&key)?))?;
        }
        (Some("pack"), [_, manifest_path, key_path, package_path]) => {
            let manifest_path = Path::new(manifest_path);
            let manifest = read_limited(manifest_path, MAX_MANIFEST_BYTES)?;
            let validated = parse_manifest(&manifest)?;
            let icon = read_icon(manifest_path, &validated)?;
            let module = read_worker_module(manifest_path, &validated)?;
            let key = read_limited(Path::new(key_path), MAX_KEY_FILE_BYTES)?;
            let package = match module {
                Some(module) => sign_worker_package(&manifest, &icon, &module, &key)?,
                None => sign_package(&manifest, &icon, &key)?,
            };
            let mut file = File::create_new(package_path)?;
            file.write_all(&package)?;
            file.sync_all()?;
        }
        (Some("verify"), [_, package_path, publisher, fingerprint, flags @ ..]) => {
            let api = api_version(flags)?;
            let trust = TrustedPublisher {
                namespace: publisher
                    .to_str()
                    .ok_or("expected a trusted publisher namespace")?,
                key_fingerprint: fingerprint_argument(fingerprint)?,
            };
            let bytes = read_limited(Path::new(package_path), MAX_PACKAGE_BYTES)?;
            let verified = verify_package(&bytes, &trust)?;
            let validated = verified.manifest();
            write_json(
                &mut output,
                &json!({
                    "compatible": is_compatible(validated, &api),
                    "manifest": validated.manifest(),
                    "packageSha256": to_hex(verified.package_digest()),
                    "publisherKeySha256": to_hex(trust.key_fingerprint),
                }),
            )?;
        }
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
