//! Origin and URL guards for the commands the messenger page may call. std-only so it is tested without the Tauri toolchain.

/// The one host this app loads. Every command refuses a caller whose page is anywhere else.
pub const APP_HOST: &str = "messager.evolvepreneuriq.app";

pub fn is_app_url(scheme: &str, host: Option<&str>) -> bool {
    scheme == "https" && host == Some(APP_HOST)
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
    fn only_the_messenger_host_over_https_is_the_app() {
        assert!(is_app_url("https", Some("messager.evolvepreneuriq.app")));
        for (s, h) in [
            ("http", Some("messager.evolvepreneuriq.app")),
            ("https", Some("evolvepreneuriq.app")),
            ("https", Some("evil.messager.evolvepreneuriq.app")),
            ("https", Some("messager.evolvepreneuriq.app.evil.test")),
            ("https", Some("MESSAGER.evolvepreneuriq.app.")),
            ("https", None),
            ("file", Some("messager.evolvepreneuriq.app")),
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
}
