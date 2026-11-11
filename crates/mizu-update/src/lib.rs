//! Signed update manifests.
//!
//! Every release publishes `mizu-update.json` and `mizu-update.json.sig`. The signature is an
//! ed25519 signature (made with Onibi Lab's offline release key) over the exact bytes of the
//! JSON file; the matching public key is compiled into Mizu. The manifest names the installer
//! and its SHA-256, so a tampered download, a tampered manifest or a manifest signed with any
//! other key is rejected before anything is executed.

use base64::Engine as _;
use base64::prelude::BASE64_STANDARD;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;

/// Onibi Lab release signing key (public half). The private key never enters the repository.
pub const RELEASE_PUBLIC_KEY: &str = "0u2eZTPr5TBpkzDuUoFAI9Y1ypf9O9myVlNmXkiFZYI=";

pub const MANIFEST_URL: &str = "https://github.com/senzukisu/mizu/releases/latest/download/mizu-update.json";
pub const SIGNATURE_URL: &str = "https://github.com/senzukisu/mizu/releases/latest/download/mizu-update.json.sig";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    /// HTTPS URL of the installer (`MizuSetup-x.y.z.exe`).
    pub url: String,
    /// Lower-case hex SHA-256 of the installer.
    pub sha256: String,
    pub size: u64,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub published: String,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum UpdateError {
    #[error("the update signature is not valid")]
    BadSignature,
    #[error("the update manifest is malformed: {0}")]
    BadManifest(String),
    #[error("the downloaded installer does not match the signed checksum")]
    ChecksumMismatch,
    #[error("the update does not come from a trusted address")]
    UntrustedUrl,
}

fn public_key(b64: &str) -> Result<VerifyingKey, UpdateError> {
    let bytes = BASE64_STANDARD.decode(b64.trim()).map_err(|_| UpdateError::BadSignature)?;
    let arr: [u8; 32] = bytes.try_into().map_err(|_| UpdateError::BadSignature)?;
    VerifyingKey::from_bytes(&arr).map_err(|_| UpdateError::BadSignature)
}

/// Verifies `signature_b64` over `manifest_bytes` with `key_b64` and parses the manifest.
pub fn verify_with_key(manifest_bytes: &[u8], signature_b64: &str, key_b64: &str) -> Result<Manifest, UpdateError> {
    let key = public_key(key_b64)?;
    let sig_bytes = BASE64_STANDARD.decode(signature_b64.trim()).map_err(|_| UpdateError::BadSignature)?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|_| UpdateError::BadSignature)?;
    key.verify(manifest_bytes, &sig).map_err(|_| UpdateError::BadSignature)?;
    let m: Manifest = serde_json::from_slice(manifest_bytes).map_err(|e| UpdateError::BadManifest(e.to_string()))?;
    if !m.url.starts_with("https://github.com/senzukisu/mizu/releases/download/") {
        return Err(UpdateError::UntrustedUrl);
    }
    if m.sha256.len() != 64 || !m.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(UpdateError::BadManifest("invalid sha256".into()));
    }
    Ok(m)
}

/// Verifies a manifest against the compiled-in release key.
pub fn verify(manifest_bytes: &[u8], signature_b64: &str) -> Result<Manifest, UpdateError> {
    verify_with_key(manifest_bytes, signature_b64, RELEASE_PUBLIC_KEY)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

pub fn check_installer(bytes: &[u8], manifest: &Manifest) -> Result<(), UpdateError> {
    if bytes.len() as u64 != manifest.size || !sha256_hex(bytes).eq_ignore_ascii_case(&manifest.sha256) {
        return Err(UpdateError::ChecksumMismatch);
    }
    Ok(())
}

/// Semantic-version comparison of `x.y.z` (pre-release suffixes sort before the release).
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let split = |v: &str| {
        let v = v.trim().trim_start_matches('v');
        let (core, pre) = v.split_once('-').map_or((v, ""), |(c, p)| (c, p));
        let nums: Vec<u64> = core.split('.').map(|p| p.parse().unwrap_or(0)).collect();
        (nums, pre.to_owned())
    };
    let (na, pa) = split(a);
    let (nb, pb) = split(b);
    for i in 0..na.len().max(nb.len()) {
        let ord = na.get(i).unwrap_or(&0).cmp(nb.get(i).unwrap_or(&0));
        if ord != Ordering::Equal {
            return ord;
        }
    }
    match (pa.is_empty(), pb.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        _ => pa.cmp(&pb),
    }
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    compare_versions(candidate, current) == Ordering::Greater
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn keypair() -> (SigningKey, String) {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk = BASE64_STANDARD.encode(sk.verifying_key().to_bytes());
        (sk, pk)
    }

    fn manifest() -> Vec<u8> {
        serde_json::to_vec(&Manifest {
            version: "0.2.0".into(),
            url: "https://github.com/senzukisu/mizu/releases/download/v0.2.0/MizuSetup-0.2.0.exe".into(),
            sha256: sha256_hex(b"installer"),
            size: 9,
            notes: String::new(),
            published: String::new(),
        })
        .unwrap()
    }

    #[test]
    fn accepts_valid_signature() {
        let (sk, pk) = keypair();
        let m = manifest();
        let sig = BASE64_STANDARD.encode(sk.sign(&m).to_bytes());
        let parsed = verify_with_key(&m, &sig, &pk).unwrap();
        assert_eq!(parsed.version, "0.2.0");
        assert!(check_installer(b"installer", &parsed).is_ok());
        assert_eq!(check_installer(b"installeR", &parsed), Err(UpdateError::ChecksumMismatch));
    }

    #[test]
    fn rejects_tampering_and_other_keys() {
        let (sk, pk) = keypair();
        let m = manifest();
        let sig = BASE64_STANDARD.encode(sk.sign(&m).to_bytes());
        let mut tampered = m.clone();
        tampered[10] ^= 1;
        assert_eq!(verify_with_key(&tampered, &sig, &pk), Err(UpdateError::BadSignature));
        let other = SigningKey::from_bytes(&[9u8; 32]);
        let other_sig = BASE64_STANDARD.encode(other.sign(&m).to_bytes());
        assert_eq!(verify_with_key(&m, &other_sig, &pk), Err(UpdateError::BadSignature));
        assert_eq!(verify_with_key(&m, "garbage", &pk), Err(UpdateError::BadSignature));
    }

    #[test]
    fn rejects_foreign_download_urls() {
        let (sk, pk) = keypair();
        let mut m: Manifest = serde_json::from_slice(&manifest()).unwrap();
        m.url = "https://evil.example/MizuSetup.exe".into();
        let bytes = serde_json::to_vec(&m).unwrap();
        let sig = BASE64_STANDARD.encode(sk.sign(&bytes).to_bytes());
        assert_eq!(verify_with_key(&bytes, &sig, &pk), Err(UpdateError::UntrustedUrl));
    }

    #[test]
    fn versions() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(is_newer("0.2.0", "0.2.0-beta.1"));
        assert!(!is_newer("v0.1.0", "0.1.1"));
    }
}
