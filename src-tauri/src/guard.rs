//! Origin and URL guards for the commands the messenger page may call. std-only so it is tested without the Tauri toolchain.

/// The one host this app loads (1.0.1: the main host, where the desktop Team Chat page lives). Every command refuses a caller whose page is anywhere else.
pub const APP_HOST: &str = "evolvepreneuriq.app";

pub fn is_app_url(scheme: &str, host: Option<&str>) -> bool {
    scheme == "https" && host == Some(APP_HOST)
}

/// Top-level navigations the window may make: the app host over https (and about:blank). Everything else is refused; links to other sites go to the system browser.
pub fn is_navigation_allowed(scheme: &str, host: Option<&str>) -> bool {
    is_app_url(scheme, host) || (scheme == "about" && host.is_none())
}

/// A download's file name made safe for the Downloads folder: no path parts, no control or reserved characters, never empty, at most 120 characters.
pub fn safe_file_name(raw: &str) -> String {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = base.chars().map(|c| if c.is_control() || "<>:\"|?*".contains(c) { '_' } else { c }).collect();
    let cleaned = cleaned.trim().trim_matches('.').to_string();
    let cleaned: String = cleaned.chars().take(120).collect();
    if cleaned.is_empty() { "download".to_string() } else { cleaned }
}

/// Links the page may ask the system browser to open: http, https and mailto only, no whitespace or control characters, at most 2048 characters.
pub fn is_openable_external(url: &str) -> bool {
    if url.is_empty() || url.len() > 2048 || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let lower = url.to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_main_host_over_https_is_the_app() {
        assert!(is_app_url("https", Some("evolvepreneuriq.app")));
        for (s, h) in [
            ("http", Some("evolvepreneuriq.app")),
            ("https", Some("messager.evolvepreneuriq.app")),
            ("https", Some("evil.evolvepreneuriq.app")),
            ("https", Some("evolvepreneuriq.app.evil.test")),
            ("https", Some("EVOLVEPRENEURIQ.app.")),
            ("https", None),
            ("file", Some("evolvepreneuriq.app")),
        ] {
            assert!(!is_app_url(s, h), "{} {:?}", s, h);
        }
    }

    #[test]
    fn external_links_are_http_https_or_mailto_and_nothing_else() {
        for ok in ["https://example.com/a?b=c", "http://example.com", "mailto:a@b.test", "HTTPS://EXAMPLE.COM"] {
            assert!(is_openable_external(ok), "{}", ok);
        }
        for bad in ["", "javascript:alert(1)", "file:///etc/passwd", "ftp://x", "tel:123", "data:text/html,x", "https://a b", "https://a\nb", "//example.com", "evolve://x"] {
            assert!(!is_openable_external(bad), "{:?}", bad);
        }
        assert!(!is_openable_external(&format!("https://{}", "a".repeat(2050))));
    }

    #[test]
    fn the_window_may_only_navigate_within_the_app_host() {
        assert!(is_navigation_allowed("https", Some("evolvepreneuriq.app")));
        assert!(is_navigation_allowed("about", None));
        for (s, h) in [("https", Some("example.com")), ("http", Some("evolvepreneuriq.app")), ("file", None), ("javascript", None), ("data", None)] {
            assert!(!is_navigation_allowed(s, h), "{s} {h:?}");
        }
    }

    #[test]
    fn download_names_are_made_safe() {
        assert_eq!(safe_file_name("report.pdf"), "report.pdf");
        assert_eq!(safe_file_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_file_name("C:\\Users\\x\\evil.exe"), "evil.exe");
        assert_eq!(safe_file_name("a<b>c:d|e?.txt"), "a_b_c_d_e_.txt");
        assert_eq!(safe_file_name(""), "download");
        assert_eq!(safe_file_name("..."), "download");
        assert_eq!(safe_file_name("   "), "download");
        assert_eq!(safe_file_name(&"x".repeat(300)).len(), 120);
    }
}
