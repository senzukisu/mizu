//! Password encryption with Windows DPAPI and CSV import/export.
//!
//! DPAPI ties the encryption key to the Windows user account: the profile database copied to
//! another machine or user cannot be decrypted. No custom cryptography is involved.

use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};

/// Extra entropy so other DPAPI users of this account cannot decrypt Mizu's blobs by accident.
const ENTROPY: &[u8] = b"Onibi Lab Mizu password store v1";

fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 }
}

fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
    // SAFETY: DPAPI allocated `cbData` bytes with LocalAlloc; we copy then free them once.
    unsafe {
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(out.pbData.cast())));
        v
    }
}

pub fn encrypt(plain: &str) -> Result<Vec<u8>, String> {
    let input = blob(plain.as_bytes());
    let entropy = blob(ENTROPY);
    let mut out = CRYPT_INTEGER_BLOB::default();
    // SAFETY: input/entropy point to live buffers for the duration of the call.
    unsafe { CryptProtectData(&input, None, Some(&entropy), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out) }
        .map_err(|e| format!("could not encrypt: {e}"))?;
    Ok(take(out))
}

pub fn decrypt(secret: &[u8]) -> Result<String, String> {
    let input = blob(secret);
    let entropy = blob(ENTROPY);
    let mut out = CRYPT_INTEGER_BLOB::default();
    // SAFETY: as above.
    unsafe { CryptUnprotectData(&input, None, Some(&entropy), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out) }
        .map_err(|e| format!("could not decrypt (different Windows user?): {e}"))?;
    String::from_utf8(take(out)).map_err(|_| "corrupt password".into())
}

/// One row of an imported password file.
#[derive(Debug, Clone, PartialEq)]
pub struct CsvLogin {
    pub origin: String,
    pub username: String,
    pub password: String,
    pub note: String,
}

/// Parses RFC 4180 CSV (quoted fields, doubled quotes, newlines inside quotes).
fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            (true, '"') => quoted = false,
            (true, c) => field.push(c),
            (false, '"') if field.is_empty() => quoted = true,
            (false, ',') => row.push(std::mem::take(&mut field)),
            (false, '\r') => {}
            (false, '\n') => {
                row.push(std::mem::take(&mut field));
                if row.iter().any(|f| !f.is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            (false, c) => field.push(c),
        }
    }
    row.push(field);
    if row.iter().any(|f| !f.is_empty()) {
        rows.push(row);
    }
    rows
}

/// Reads password exports from Chrome, Edge, Brave, Opera, Vivaldi (`name,url,username,password,note`),
/// Firefox (`url,username,password,httpRealm,formActionOrigin,…`) and Bitwarden
/// (`…,login_uri,login_username,login_password`).
pub fn import_csv(text: &str) -> Result<Vec<CsvLogin>, String> {
    let rows = parse_csv(text);
    let header: Vec<String> = rows.first().ok_or("the file is empty")?.iter().map(|h| h.trim().to_ascii_lowercase()).collect();
    let col = |names: &[&str]| header.iter().position(|h| names.contains(&h.as_str()));
    let url = col(&["url", "login_uri", "origin", "website", "web site", "hostname"]).ok_or("no URL column found")?;
    let user = col(&["username", "login_username", "user", "login", "email", "user name"]).ok_or("no username column found")?;
    let pass = col(&["password", "login_password"]).ok_or("no password column found")?;
    let note = col(&["note", "notes", "extra", "comment"]);
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        let get = |i: usize| r.get(i).map(|s| s.trim().to_owned()).unwrap_or_default();
        let raw_url = get(url);
        let password = r.get(pass).cloned().unwrap_or_default();
        if raw_url.is_empty() || password.is_empty() {
            continue;
        }
        let with_scheme = if raw_url.contains("://") { raw_url } else { format!("https://{raw_url}") };
        let Some(origin) = mizu_core::omnibox::origin_of(&with_scheme) else { continue };
        if !(origin.starts_with("https://") || origin.starts_with("http://")) {
            continue; // android://, chrome-extension:// …
        }
        out.push(CsvLogin { origin, username: get(user), password, note: note.map(get).unwrap_or_default() });
    }
    Ok(out)
}

/// Quotes a CSV field when needed. Values are never altered: a password export must
/// round-trip exactly (so no spreadsheet-formula escaping either).
fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) || s.starts_with(' ') || s.ends_with(' ') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

/// Chrome-compatible export (imports into Chrome, Edge, Firefox, Bitwarden, 1Password…).
pub fn export_csv(rows: &[CsvLogin]) -> String {
    let mut out = String::from("name,url,username,password,note\r\n");
    for r in rows {
        let name = mizu_core::omnibox::host_of(&r.origin).unwrap_or_default();
        out.push_str(&[csv_field(&name), csv_field(&r.origin), csv_field(&r.username), csv_field(&r.password), csv_field(&r.note)].join(","));
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpapi_round_trip() {
        let secret = encrypt("hunter2 — пароль").unwrap();
        assert!(!secret.windows(7).any(|w| w == b"hunter2"));
        assert_eq!(decrypt(&secret).unwrap(), "hunter2 — пароль");
        assert!(decrypt(b"garbage").is_err());
    }

    #[test]
    fn imports_chrome_firefox_bitwarden() {
        let chrome = "name,url,username,password,note\nexample.com,https://example.com/login,alice,\"p,a\"\"ss\",hi\n";
        let r = import_csv(chrome).unwrap();
        assert_eq!(r[0], CsvLogin { origin: "https://example.com".into(), username: "alice".into(), password: "p,a\"ss".into(), note: "hi".into() });

        let firefox = "\"url\",\"username\",\"password\",\"httpRealm\"\r\n\"https://site.example\",\"bob\",\"pw\",\"\"\r\n";
        assert_eq!(import_csv(firefox).unwrap()[0].username, "bob");

        let bw = "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\n,,login,X,n,,0,site.example,carol,pw2,\n";
        let r = import_csv(bw).unwrap();
        assert_eq!(r[0].origin, "https://site.example");
        assert_eq!(r[0].password, "pw2");

        assert!(import_csv("a,b\n1,2\n").is_err());
        assert!(import_csv("url,username,password\nandroid://x,u,p\n").unwrap().is_empty());
    }

    #[test]
    fn export_is_reimportable() {
        let rows = vec![CsvLogin { origin: "https://a.example".into(), username: "u".into(), password: "=cmd,\"x\"".into(), note: String::new() }];
        let csv = export_csv(&rows);
        let back = import_csv(&csv).unwrap();
        assert_eq!(back[0].password, "=cmd,\"x\"");
    }
}
