//! Origin and URL guards for the commands the messenger page may call. std-only so it is tested without the Tauri toolchain.

/// The one host this app loads (1.0.1: the main host, where the desktop Team Chat page lives). Every command refuses a caller whose page is anywhere else.
pub const APP_HOST: &str = "messager.evolvepreneuriq.app";

pub fn is_app_url(scheme: &str, host: Option<&str>) -> bool {
    scheme == "https" && host == Some(APP_HOST)
}

/// Pages of the app host that belong in THIS window: the chat and the sign-in pages. Any other page of the site (a contact, an invoice, the dashboard) opens in the system browser instead,
/// so the chat window never turns into the whole EIQ app.
// The first path words of the phone Messenger host's pages (MOBILE APPS, 11 Nov): clicks on these stay in the window, everything else opens in the system browser.
pub const IN_WINDOW: [&str; 21] = [
    "app", "apps", "app-components", "login", "login_check", "logout", "switch", "messenger", "chat", "chatbot", "workspace", "communication-hub", "email-manager", "va-api", "api", "data-tables",
    "books", "2fa", "forgot", "reset", "register",
];

pub fn is_in_window_path(path: &str) -> bool {
    let first = path.trim_start_matches('/').split(['/', '?', '#']).next().unwrap_or("");
    IN_WINDOW.contains(&first)
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

    #[test]
    fn the_window_may_only_navigate_within_the_app_host() {
        assert!(is_navigation_allowed("https", Some("messager.evolvepreneuriq.app")));
        assert!(is_navigation_allowed("about", None));
        for (s, h) in [("https", Some("example.com")), ("http", Some("messager.evolvepreneuriq.app")), ("https", Some("evolvepreneuriq.app")), ("file", None), ("javascript", None), ("data", None)] {
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

    #[test]
    fn only_the_chat_and_sign_in_pages_stay_in_the_window() {
        for p in ["/chat", "/chat/desktop", "/chat?embed=1", "/chat/channel/5", "/login", "/login/", "/login_check", "/2fa/check", "/logout", "/forgot", "/reset/abc", "/register", "/app/13", "/switch/5", "/messenger/api/badge", "/communication-hub/x"] {
            assert!(is_in_window_path(p), "{p}");
        }
        for p in ["/", "/crm/contacts/5", "/dashboard", "/chatter", "/loginfo", "/admin/chat", "/appx", "/switcher", "/admin/lab", ""] {
            assert!(!is_in_window_path(p), "{p}");
        }
    }
}
