//! URL matching for `@match` (Chrome match patterns) and `@include`/`@exclude` (globs or
//! `/regex/`). Regex includes are supported for the common subset (anchors, `.`, `*`, `+`,
//! `?`, character classes, escapes, groups with alternation) via a tiny backtracking matcher,
//! so no regex dependency is needed for this crate.

/// Chrome-style match pattern: `<scheme>://<host><path>`, e.g. `https://*.example.com/*`.
pub fn match_pattern(pattern: &str, url: &str) -> bool {
    if pattern == "<all_urls>" {
        return url.starts_with("http://") || url.starts_with("https://") || url.starts_with("file://");
    }
    let Ok(parsed) = url::Url::parse(url) else { return false };
    let Some((scheme, rest)) = pattern.split_once("://") else { return false };
    let scheme_ok = match scheme {
        "*" => matches!(parsed.scheme(), "http" | "https"),
        s => s == parsed.scheme(),
    };
    if !scheme_ok {
        return false;
    }
    let (host_pat, path_pat) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let host = parsed.host_str().unwrap_or("");
    let host_pat = host_pat.split(':').next().unwrap_or(host_pat);
    let host_ok = if host_pat == "*" {
        true
    } else if let Some(base) = host_pat.strip_prefix("*.") {
        host == base || host.ends_with(&format!(".{base}"))
    } else {
        host.eq_ignore_ascii_case(host_pat)
    };
    if !host_ok {
        return false;
    }
    let mut path = parsed.path().to_owned();
    if let Some(q) = parsed.query() {
        path.push('?');
        path.push_str(q);
    }
    glob(path_pat, &path)
}

/// Greasemonkey `@include` semantics: `*` matches anything; `/…/` is a regular expression.
pub fn include_pattern(pattern: &str, url: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.len() > 2 && pattern.starts_with('/') && pattern.ends_with('/') {
        return regex_lite::is_match(&pattern[1..pattern.len() - 1], url);
    }
    // Greasemonkey's `.tld` magic: treat it as "any top-level domain".
    let pattern = pattern.replace(".tld/", ".*/");
    glob(&pattern, url)
}

/// `*` matches any run of characters; everything else literally.
pub fn glob(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0, 0);
    let (mut star, mut mark) = (None, 0);
    while ti < t.len() {
        if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

mod regex_lite {
    //! Minimal backtracking regex for userscript `@include /…/` rules.

    #[derive(Debug, Clone)]
    enum Node {
        Char(char),
        Any,
        Class(Vec<(char, char)>, bool),
        Group(Vec<Vec<Node>>),
        Start,
        End,
        Repeat(Box<Node>, usize, usize),
    }

    pub fn is_match(pattern: &str, text: &str) -> bool {
        let chars: Vec<char> = pattern.chars().collect();
        let mut pos = 0;
        let Some(alts) = parse_alts(&chars, &mut pos) else { return false };
        let t: Vec<char> = text.chars().collect();
        (0..=t.len()).any(|start| match_alts(&alts, &t, start, &mut |_| true))
    }

    fn parse_alts(p: &[char], pos: &mut usize) -> Option<Vec<Vec<Node>>> {
        let mut alts = vec![Vec::new()];
        while *pos < p.len() {
            let c = p[*pos];
            match c {
                ')' => break,
                '|' => {
                    *pos += 1;
                    alts.push(Vec::new());
                    continue;
                }
                _ => {}
            }
            let atom = parse_atom(p, pos)?;
            let atom = parse_quant(p, pos, atom);
            alts.last_mut()?.push(atom);
        }
        Some(alts)
    }

    fn parse_atom(p: &[char], pos: &mut usize) -> Option<Node> {
        let c = p[*pos];
        *pos += 1;
        Some(match c {
            '.' => Node::Any,
            '^' => Node::Start,
            '$' => Node::End,
            '(' => {
                if p.get(*pos) == Some(&'?') && p.get(*pos + 1) == Some(&':') {
                    *pos += 2;
                }
                let alts = parse_alts(p, pos)?;
                if p.get(*pos) != Some(&')') {
                    return None;
                }
                *pos += 1;
                Node::Group(alts)
            }
            '[' => {
                let negate = p.get(*pos) == Some(&'^');
                if negate {
                    *pos += 1;
                }
                let mut ranges = Vec::new();
                while *pos < p.len() && p[*pos] != ']' {
                    let mut lo = p[*pos];
                    *pos += 1;
                    if lo == '\\' && *pos < p.len() {
                        lo = p[*pos];
                        *pos += 1;
                        if let Some(r) = class_escape(lo) {
                            ranges.extend(r);
                            continue;
                        }
                    }
                    if p.get(*pos) == Some(&'-') && p.get(*pos + 1).is_some_and(|&c| c != ']') {
                        let hi = p[*pos + 1];
                        *pos += 2;
                        ranges.push((lo, hi));
                    } else {
                        ranges.push((lo, lo));
                    }
                }
                *pos += 1;
                Node::Class(ranges, negate)
            }
            '\\' => {
                let e = *p.get(*pos)?;
                *pos += 1;
                match class_escape(e) {
                    Some(r) => Node::Class(r, false),
                    None => match e {
                        'D' => Node::Class(vec![('0', '9')], true),
                        'W' => Node::Class(word(), true),
                        'S' => Node::Class(space(), true),
                        _ => Node::Char(e),
                    },
                }
            }
            other => Node::Char(other),
        })
    }

    fn word() -> Vec<(char, char)> {
        vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')]
    }
    fn space() -> Vec<(char, char)> {
        vec![(' ', ' '), ('\t', '\t'), ('\n', '\n'), ('\r', '\r')]
    }
    fn class_escape(e: char) -> Option<Vec<(char, char)>> {
        match e {
            'd' => Some(vec![('0', '9')]),
            'w' => Some(word()),
            's' => Some(space()),
            _ => None,
        }
    }

    fn parse_quant(p: &[char], pos: &mut usize, atom: Node) -> Node {
        let (min, max) = match p.get(*pos) {
            Some('*') => (0, usize::MAX),
            Some('+') => (1, usize::MAX),
            Some('?') => (0, 1),
            _ => return atom,
        };
        *pos += 1;
        if p.get(*pos) == Some(&'?') {
            *pos += 1; // lazy quantifiers behave the same for a yes/no match
        }
        Node::Repeat(Box::new(atom), min, max)
    }

    fn match_alts(alts: &[Vec<Node>], t: &[char], i: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        alts.iter().any(|seq| match_seq(seq, t, i, k))
    }

    fn match_seq(seq: &[Node], t: &[char], i: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        let Some((first, rest)) = seq.split_first() else { return k(i) };
        match_node(first, t, i, &mut |j| match_seq(rest, t, j, k))
    }

    fn match_node(n: &Node, t: &[char], i: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        match n {
            Node::Char(c) => t.get(i) == Some(c) && k(i + 1),
            Node::Any => i < t.len() && k(i + 1),
            Node::Class(ranges, neg) => t.get(i).is_some_and(|c| {
                ranges.iter().any(|(lo, hi)| (lo..=hi).contains(&c)) != *neg
            }) && k(i + 1),
            Node::Start => i == 0 && k(i),
            Node::End => i == t.len() && k(i),
            Node::Group(alts) => match_alts(alts, t, i, k),
            Node::Repeat(inner, min, max) => repeat(inner, *min, *max, 0, t, i, k),
        }
    }

    fn repeat(
        inner: &Node,
        min: usize,
        max: usize,
        count: usize,
        t: &[char],
        i: usize,
        k: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        // Greedy: try one more repetition first, then stop.
        if count < max
            && match_node(inner, t, i, &mut |j| {
                j > i && repeat(inner, min, max, count + 1, t, j, k)
            })
        {
            return true;
        }
        count >= min && k(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_patterns() {
        assert!(match_pattern("https://*.example.com/*", "https://www.example.com/a"));
        assert!(match_pattern("https://*.example.com/*", "https://example.com/"));
        assert!(!match_pattern("https://*.example.com/*", "https://badexample.com/"));
        assert!(match_pattern("*://example.com/foo*", "http://example.com/foobar?x=1"));
        assert!(!match_pattern("*://example.com/foo*", "ftp://example.com/foo"));
        assert!(match_pattern("<all_urls>", "https://anything/"));
        assert!(!match_pattern("https://example.com/*", "https://example.org/"));
    }

    #[test]
    fn include_globs_and_regex() {
        assert!(include_pattern("*", "https://x/"));
        assert!(include_pattern("https://*.google.tld/*", "https://www.google.de/search"));
        assert!(include_pattern(r"/^https://music\.youtube\.com/", "https://music.youtube.com/watch"));
        assert!(!include_pattern(r"/^https://music\.youtube\.com/", "https://www.youtube.com/"));
        assert!(include_pattern(r"/^https?://(www\.)?reddit\.com/r/[a-z]+/", "http://reddit.com/r/rust/"));
    }
}
