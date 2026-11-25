//! Windows import identity at rest, bound to the current user through DPAPI.
use crate::core::{AppError, AppResult};
use std::{
    fs::File,
    io::{ErrorKind, Read, Write},
    path::Path,
    ptr,
};
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    },
};
use zeroize::{Zeroize, Zeroizing};

pub(crate) struct ImportIdentity {
    pub certificate: CertificateDer<'static>,
    pub private_key: PrivatePkcs8KeyDer<'static>,
}

const MAX_PROTECTED_BYTES: u64 = 16 * 1024;

/// Opens the current identity or atomically creates the first one. A corrupt or inaccessible
/// identity is reported so existing clients never silently switch to a different trust anchor.
pub(crate) fn load_or_create(directory: &Path) -> AppResult<ImportIdentity> {
    let path = directory.join("import-tls-identity.dpapi");
    match read(&path) {
        Ok(identity) => return Ok(identity),
        Err(AppError::Io(error)) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let generated = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()])
        .map_err(|_| AppError::Internal("Could not generate the import TLS identity".into()))?;
    let certificate = generated.cert.der();
    let private_key = Zeroizing::new(generated.key_pair.serialize_der());
    let mut serialized = Zeroizing::new(Vec::new());
    serialized.extend_from_slice(b"MUI1");
    let length = (certificate.as_ref().len() as u32).to_be_bytes();
    serialized.extend_from_slice(&length);
    serialized.extend_from_slice(certificate.as_ref());
    serialized.extend_from_slice(&private_key);
    let protected = crypt(&serialized, true)?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(&protected)?;
    temporary.as_file().sync_all()?;
    match temporary.persist_noclobber(&path) {
        Ok(_) => Ok(ImportIdentity {
            certificate: certificate.clone(),
            private_key: PrivatePkcs8KeyDer::from(private_key.to_vec()),
        }),
        Err(error) if error.error.kind() == ErrorKind::AlreadyExists => read(&path),
        Err(error) => Err(error.error.into()),
    }
}

fn read(path: &Path) -> AppResult<ImportIdentity> {
    let mut protected = Vec::new();
    File::open(path)?
        .take(MAX_PROTECTED_BYTES + 1)
        .read_to_end(&mut protected)?;
    if protected.is_empty() || protected.len() as u64 > MAX_PROTECTED_BYTES {
        return Err(invalid_identity());
    }
    let data = crypt(&protected, false)?;
    let payload = data.strip_prefix(b"MUI1").ok_or_else(invalid_identity)?;
    let (length, payload) = payload.split_at_checked(4).ok_or_else(invalid_identity)?;
    let length = u32::from_be_bytes(length.try_into().map_err(|_| invalid_identity())?) as usize;
    let (certificate, private_key) = payload
        .split_at_checked(length)
        .ok_or_else(invalid_identity)?;
    if certificate.is_empty() || private_key.is_empty() {
        return Err(invalid_identity());
    }
    Ok(ImportIdentity {
        certificate: CertificateDer::from(certificate.to_vec()),
        private_key: PrivatePkcs8KeyDer::from(private_key.to_vec()),
    })
}

fn crypt(input: &[u8], protect: bool) -> AppResult<Zeroizing<Vec<u8>>> {
    let data = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(input.len()).map_err(|_| invalid_identity())?,
        pbData: input.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };
    // DPAPI reads the input and allocates output with LocalAlloc on success. No machine-wide
    // flag is set, so only this user's profile can decrypt it. Never allow a system prompt.
    let success = unsafe {
        if protect {
            CryptProtectData(
                &data,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &data,
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if success == 0 {
        return Err(invalid_identity());
    }
    if output.pbData.is_null() {
        return Err(invalid_identity());
    }
    // The successful call owns a writable buffer of cbData bytes until LocalFree.
    let data = unsafe { std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize) };
    let result = Zeroizing::new(data.to_vec());
    data.zeroize();
    unsafe {
        LocalFree(output.pbData.cast());
    }
    Ok(result)
}

fn invalid_identity() -> AppError {
    AppError::ValidationFailed("The local import identity could not be opened".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpapi_identity_is_stable_and_corruption_never_rotates_it() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let first = load_or_create(directory.path())?;
        let second = load_or_create(directory.path())?;
        assert_eq!(first.certificate.as_ref(), second.certificate.as_ref());
        assert!(first.private_key.secret_pkcs8_der() == second.private_key.secret_pkcs8_der());
        let path = directory.path().join("import-tls-identity.dpapi");
        let file = std::fs::read(&path)?;
        assert!(!file
            .windows(first.private_key.secret_pkcs8_der().len())
            .any(|window| window == first.private_key.secret_pkcs8_der()));
        std::fs::write(&path, b"corrupted test identity")?;
        assert!(load_or_create(directory.path()).is_err());
        assert_eq!(std::fs::read(path)?, b"corrupted test identity");
        Ok(())
    }
}
