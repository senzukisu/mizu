//! HTTPS-First: a top-level `http://` address is tried over `https://` first. Local addresses
//! (IPs, `localhost`, single-label and home-network names, explicit ports) are left alone, as
//! are sites the user allowed over HTTP.

/// The site name used for exceptions: lower case, without "www.".
pub fn site(host: &str) -> String {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    host.strip_prefix("www.").unwrap_or(&host).to_owned()
}

/// `https://` version of `uri` when HTTPS-First should try it, `None` to load it as it is.
pub fn upgrade_target(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("http://")?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..end];
    if authority.contains('@') {
        return None;
    }
    // IPv6 literals and explicit ports: local services, often without HTTPS.
    if authority.starts_with('[') || authority.contains(':') {
        return None;
    }
    let host = authority.to_ascii_lowercase();
    if is_local(&host) {
        return None;
    }
    Some(format!("https://{rest}"))
}

/// Hosts that are almost never served over HTTPS: the machine itself, the home network.
pub fn is_local(host: &str) -> bool {
    let host = host.trim_end_matches('.');
    if host.is_empty() || !host.contains('.') || host.parse::<std::net::Ipv4Addr>().is_ok() {
        return true;
    }
    const LOCAL: [&str; 9] = [".localhost", ".local", ".lan", ".home", ".internal", ".home.arpa", ".test", ".intranet", ".corp"];
    host == "localhost" || LOCAL.iter().any(|s| host.ends_with(s))
}

/// Navigation errors that mean "this site has no working HTTPS" (anything but a cancel).
pub fn is_failure(web_error: i32) -> bool {
    // COREWEBVIEW2_WEB_ERROR_STATUS_OPERATION_CANCELED
    web_error != 14
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrades_public_sites_only() {
        assert_eq!(upgrade_target("http://example.com/a?b#c").as_deref(), Some("https://example.com/a?b#c"));
        assert_eq!(upgrade_target("http://Example.org").as_deref(), Some("https://Example.org"));
        assert_eq!(upgrade_target("https://example.com/"), None);
        assert_eq!(upgrade_target("http://localhost:3000/"), None);
        assert_eq!(upgrade_target("http://192.168.1.1/"), None);
        assert_eq!(upgrade_target("http://router/"), None);
        assert_eq!(upgrade_target("http://nas.local/"), None);
        assert_eq!(upgrade_target("http://example.com:8080/"), None);
        assert_eq!(upgrade_target("http://[::1]/"), None);
        assert_eq!(upgrade_target("http://user@example.com/"), None);
        assert_eq!(site("WWW.Example.com"), "example.com");
        assert!(!is_failure(14));
        assert!(is_failure(12));
    }
}
