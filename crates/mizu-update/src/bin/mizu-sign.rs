//! Release signing tool (run by the maintainer, never shipped).
//!
//! ```text
//! mizu-sign keygen <private-key-file>                  create a key pair, print the public key
//! mizu-sign manifest <private-key-file> <installer> <version> <out-dir> [notes]
//! ```
//! The private key file must be kept outside the repository (and backed up offline).

use base64::Engine as _;
use base64::prelude::BASE64_STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use mizu_update::{Manifest, sha256_hex};
use std::path::Path;

fn load_key(path: &str) -> SigningKey {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| fail(&format!("cannot read key {path}: {e}")));
    let bytes = BASE64_STANDARD.decode(text.trim()).unwrap_or_else(|_| fail("key file is not base64"));
    let arr: [u8; 32] = bytes.try_into().unwrap_or_else(|_| fail("key must be 32 bytes"));
    SigningKey::from_bytes(&arr)
}

fn fail(msg: &str) -> ! {
    eprintln!("mizu-sign: {msg}");
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("keygen") => {
            let path = args.get(2).unwrap_or_else(|| fail("usage: mizu-sign keygen <file>"));
            if Path::new(path).exists() {
                fail("refusing to overwrite an existing key");
            }
            let mut seed = [0u8; 32];
            getrandom::getrandom(&mut seed).unwrap_or_else(|e| fail(&format!("no randomness: {e}")));
            let key = SigningKey::from_bytes(&seed);
            if let Some(dir) = Path::new(path).parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            std::fs::write(path, BASE64_STANDARD.encode(seed)).unwrap_or_else(|e| fail(&e.to_string()));
            println!("{}", BASE64_STANDARD.encode(key.verifying_key().to_bytes()));
        }
        Some("manifest") => {
            if args.len() < 6 {
                fail("usage: mizu-sign manifest <key> <installer> <version> <out-dir> [notes]");
            }
            let key = load_key(&args[2]);
            let installer = std::fs::read(&args[3]).unwrap_or_else(|e| fail(&format!("cannot read installer: {e}")));
            let version = args[4].trim_start_matches('v').to_owned();
            let file = Path::new(&args[3]).file_name().unwrap_or_default().to_string_lossy().into_owned();
            let manifest = Manifest {
                url: format!("https://github.com/senzukisu/mizu/releases/download/v{version}/{file}"),
                sha256: sha256_hex(&installer),
                size: installer.len() as u64,
                notes: args.get(6).cloned().unwrap_or_default(),
                published: String::new(),
                version,
            };
            let json = serde_json::to_vec_pretty(&manifest).unwrap_or_else(|e| fail(&e.to_string()));
            let sig = BASE64_STANDARD.encode(key.sign(&json).to_bytes());
            let out = Path::new(&args[5]);
            std::fs::create_dir_all(out).unwrap_or_else(|e| fail(&e.to_string()));
            std::fs::write(out.join("mizu-update.json"), &json).unwrap_or_else(|e| fail(&e.to_string()));
            std::fs::write(out.join("mizu-update.json.sig"), &sig).unwrap_or_else(|e| fail(&e.to_string()));
            println!("signed manifest for {} ({} bytes, sha256 {})", manifest.version, manifest.size, manifest.sha256);
        }
        _ => fail("usage: mizu-sign keygen <file> | manifest <key> <installer> <version> <out-dir> [notes]"),
    }
}
