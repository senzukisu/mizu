//! Where userscripts come from: turning the links people share (repository pages, script
//! listings) into the raw `.user.js` URL, plus version comparison for updates.

use std::cmp::Ordering;

/// Download URLs to try for a link, best first. Repository "view" pages are rewritten to
/// their raw form; GitHub links get a jsDelivr mirror as fallback (raw.githubusercontent.com is
/// blocked on some networks).
pub fn install_candidates(input: &str) -> Vec<String> {
    let input = input.trim();
    let Ok(url) = url::Url::parse(input) else { return Vec::new() };
    if !matches!(url.scheme(), "https" | "http" | "file") {
        return Vec::new();
    }
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let segs: Vec<String> = url
        .path_segments()
        .map(|s| s.filter(|p| !p.is_empty()).map(str::to_owned).collect())
        .unwrap_or_default();
    let s = |i: usize| segs.get(i).map(String::as_str).unwrap_or("");
    let rest = |i: usize| segs.get(i..).map(|r| r.join("/")).unwrap_or_default();
    let mut out = Vec::new();

    match host.as_str() {
        // github.com/{user}/{repo}/blob|raw/{ref}/{path}
        "github.com" | "www.github.com" if segs.len() >= 5 && matches!(s(2), "blob" | "raw") => {
            let (user, repo, git_ref, path) = (s(0), s(1), s(3), rest(4));
            out.push(format!("https://raw.githubusercontent.com/{user}/{repo}/{git_ref}/{path}"));
            out.push(format!("https://cdn.jsdelivr.net/gh/{user}/{repo}@{git_ref}/{path}"));
        }
        // raw.githubusercontent.com/{user}/{repo}/{ref}/{path}
        "raw.githubusercontent.com" if segs.len() >= 4 => {
            out.push(input.to_owned());
            let refs_heads = s(2) == "refs" && s(3) == "heads";
            let (git_ref, path) = if refs_heads { (s(4).to_owned(), rest(5)) } else { (s(2).to_owned(), rest(3)) };
            out.push(format!("https://cdn.jsdelivr.net/gh/{}/{}@{git_ref}/{path}", s(0), s(1)));
        }
        // gist.github.com/{user}/{id}  → raw of the (single-file) gist
        "gist.github.com" if segs.len() >= 2 && s(2) != "raw" => {
            out.push(format!("https://gist.githubusercontent.com/{}/{}/raw", s(0), s(1)));
        }
        // gitlab.com/{group…}/{project}/-/blob/{ref}/{path}
        h if (h == "gitlab.com" || h.starts_with("gitlab.")) && url.path().contains("/-/blob/") => {
            out.push(input.replacen("/-/blob/", "/-/raw/", 1));
        }
        // codeberg.org/{user}/{repo}/src/branch|commit|tag/{ref}/{path}
        "codeberg.org" if s(2) == "src" => {
            out.push(input.replacen("/src/", "/raw/", 1));
        }
        // bitbucket.org/{user}/{repo}/src/{ref}/{path}
        "bitbucket.org" if s(2) == "src" => {
            out.push(input.replacen("/src/", "/raw/", 1));
        }
        // greasyfork.org/{lang?}/scripts/{id}-{slug}  (listing page, not the code)
        "greasyfork.org" | "sleazyfork.org" if !url.path().ends_with(".user.js") => {
            let pos = segs.iter().position(|p| p == "scripts");
            if let Some(id) = pos.and_then(|p| segs.get(p + 1)) {
                let id: String = id.chars().take_while(char::is_ascii_digit).collect();
                if !id.is_empty() {
                    out.push(format!("https://{host}/scripts/{id}/code/script.user.js"));
                }
            }
        }
        // openuserjs.org/scripts/{user}/{name}
        "openuserjs.org" if s(0) == "scripts" && segs.len() >= 3 => {
            out.push(format!("https://openuserjs.org/install/{}/{}.user.js", s(1), s(2).trim_end_matches(".user.js")));
        }
        _ => {}
    }
    if out.is_empty() {
        out.push(input.to_owned());
    }
    out.dedup();
    out
}

/// Splits text that may contain several userscripts back to back.
pub fn split_scripts(text: &str) -> Vec<String> {
    let marker = "==UserScript==";
    let mut starts: Vec<usize> = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        if t.starts_with("//") && t.contains(marker) && !t.contains("==/UserScript==") {
            starts.push(offset);
        }
        offset += line.len();
    }
    if starts.len() <= 1 {
        return vec![text.to_owned()];
    }
    let mut out = Vec::new();
    for (i, &start) in starts.iter().enumerate() {
        let end = starts.get(i + 1).copied().unwrap_or(text.len());
        out.push(text[start..end].to_owned());
    }
    out
}

/// Compares userscript versions the way script managers do: dot-separated parts, numeric
/// where possible (`1.10` > `1.9`), missing parts count as zero, and `1.0b` < `1.0`.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let parts = |v: &str| -> Vec<(u64, String)> {
        v.trim()
            .trim_start_matches(['v', 'V'])
            .split(['.', '-', '+'])
            .map(|p| {
                let digits: String = p.chars().take_while(char::is_ascii_digit).collect();
                let suffix = p[digits.len()..].to_owned();
                (digits.parse().unwrap_or(0), suffix)
            })
            .collect()
    };
    let (pa, pb) = (parts(a), parts(b));
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i).cloned().unwrap_or((0, String::new()));
        let y = pb.get(i).cloned().unwrap_or((0, String::new()));
        let ord = x.0.cmp(&y.0).then_with(|| match (x.1.is_empty(), y.1.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater, // 1.0 > 1.0b
            (false, true) => Ordering::Less,
            (false, false) => x.1.cmp(&y.1),
        });
        if ord != Ordering::Equal {
            return ord;
        }
    }
    Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_links() {
        let c = install_candidates("https://github.com/user/repo/blob/main/dir/x.user.js");
        assert_eq!(c[0], "https://raw.githubusercontent.com/user/repo/main/dir/x.user.js");
        assert_eq!(c[1], "https://cdn.jsdelivr.net/gh/user/repo@main/dir/x.user.js");
        let c = install_candidates("https://raw.githubusercontent.com/user/repo/refs/heads/dev/x.user.js");
        assert_eq!(c[1], "https://cdn.jsdelivr.net/gh/user/repo@dev/x.user.js");
        assert_eq!(
            install_candidates("https://gist.github.com/someone/abc123")[0],
            "https://gist.githubusercontent.com/someone/abc123/raw"
        );
    }

    #[test]
    fn other_hosts() {
        assert_eq!(
            install_candidates("https://gitlab.com/g/p/-/blob/main/s.user.js")[0],
            "https://gitlab.com/g/p/-/raw/main/s.user.js"
        );
        assert_eq!(
            install_candidates("https://codeberg.org/u/r/src/branch/main/s.user.js")[0],
            "https://codeberg.org/u/r/raw/branch/main/s.user.js"
        );
        assert_eq!(
            install_candidates("https://greasyfork.org/ru/scripts/12345-some-script")[0],
            "https://greasyfork.org/scripts/12345/code/script.user.js"
        );
        assert_eq!(
            install_candidates("https://openuserjs.org/scripts/alice/Cool_Script")[0],
            "https://openuserjs.org/install/alice/Cool_Script.user.js"
        );
        assert_eq!(install_candidates("https://example.com/a.user.js"), ["https://example.com/a.user.js"]);
        assert!(install_candidates("javascript:alert(1)").is_empty());
    }

    #[test]
    fn splits_bundles() {
        let one = "// ==UserScript==\n// @name A\n// ==/UserScript==\nA();\n";
        let two = "// ==UserScript==\n// @name B\n// ==/UserScript==\nB();\n";
        let parts = split_scripts(&format!("{one}{two}"));
        assert_eq!(parts.len(), 2);
        assert!(parts[1].contains("@name B"));
        assert_eq!(split_scripts(one).len(), 1);
    }

    #[test]
    fn versions() {
        assert_eq!(compare_versions("1.10", "1.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("2.0b", "2.0"), Ordering::Less);
        assert_eq!(compare_versions("v3.1", "3.0.9"), Ordering::Greater);
        assert_eq!(compare_versions("2024.11.10", "2024.9.30"), Ordering::Greater);
    }
}
