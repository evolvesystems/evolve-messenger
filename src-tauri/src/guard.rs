//! Origin and URL guards for the commands the messenger page may call. std-only so it is tested without the Tauri toolchain.

/// The one host this app loads (1.0.1: the main host, where the desktop Team Chat page lives). Every command refuses a caller whose page is anywhere else.
pub const APP_HOST: &str = "evolvepreneuriq.app";

pub fn is_app_url(scheme: &str, host: Option<&str>) -> bool {
    scheme == "https" && host == Some(APP_HOST)
}

/// A same-site page (1.0.8, EVO-BUG-20300): https, EXACTLY the app host, no user name or password in the address, and the default port. Parsed pieces only, never a prefix or suffix match on a string
/// (`evolvepreneuriq.app.evil.com`, `evil-evolvepreneuriq.app` and `https://evolvepreneuriq.app@evil.com/` are all NOT same-site). Every path of the app host stays in the window; EXTERNAL_LINKS_JS applies the
/// same rule to a clicked link (the cases are shared with tests/links_harness.mjs).
pub fn is_same_site(scheme: &str, host: Option<&str>, username: &str, password: Option<&str>, port: Option<u16>) -> bool {
    is_app_url(scheme, host) && username.is_empty() && password.is_none() && matches!(port, None | Some(443))
}

/// Top-level navigations the window may make: a same-site page (every redirect hop is checked, so a hop to another host is refused) and about:blank. Everything else is refused; links to other sites go to the system browser.
pub fn is_navigation_allowed(scheme: &str, host: Option<&str>, username: &str, password: Option<&str>, port: Option<u16>) -> bool {
    is_same_site(scheme, host, username, password, port) || (scheme == "about" && host.is_none())
}

/// What to do with a navigation that `is_navigation_allowed` refused (1.0.8, Auditor minors 5 and 1): a refused address on the app host itself (a stray port or user name after a redirect) takes the window back to
/// its start page instead of leaving it blank; anything else is simply refused. A refused hop to ANOTHER site is never opened in the system browser from here: a redirect or a script is not a click, and only a
/// click the person made (handled in links.js, which asks `open_external_url`) may open the browser (Auditor minor 1).
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    GoHome,
    Nothing,
}

pub fn refused_action(host: Option<&str>) -> Refusal {
    if host == Some(APP_HOST) {
        Refusal::GoHome
    } else {
        Refusal::Nothing
    }
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
    if lower.starts_with("mailto:") {
        // a mailto may not ask the mail program to attach a file from this computer (Auditor minor 3): any header named attach or attachment, however it is cased or percent-encoded
        let query = lower.split_once('?').map(|(_, q)| q).unwrap_or("");
        let decoded = query.replace("%61", "a").replace("%74", "t");
        let has_attach = |q: &str| q.split(['&', ';']).any(|kv| kv.split('=').next().map(|k| k.trim() == "attach" || k.trim() == "attachment").unwrap_or(false));
        return !(has_attach(query) || has_attach(&decoded));
    }
    lower.starts_with("https://") || lower.starts_with("http://")
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
        for ok in ["https://example.com/a?b=c", "http://example.com", "mailto:a@b.test", "HTTPS://EXAMPLE.COM", "mailto:a@b.test?subject=Hi&body=attached%20soon"] {
            assert!(is_openable_external(ok), "{}", ok);
        }
        for bad in ["", "javascript:alert(1)", "file:///etc/passwd", "ftp://x", "tel:123", "data:text/html,x", "https://a b", "https://a\nb", "//example.com", "evolve://x"] {
            assert!(!is_openable_external(bad), "{:?}", bad);
        }
        assert!(!is_openable_external(&format!("https://{}", "a".repeat(2050))));
    }

    #[test]
    fn the_window_may_only_navigate_within_the_app_host() {
        assert!(is_navigation_allowed("https", Some("evolvepreneuriq.app"), "", None, None));
        assert!(is_navigation_allowed("about", None, "", None, None));
        for (s, h) in [("https", Some("example.com")), ("http", Some("evolvepreneuriq.app")), ("file", None), ("javascript", None), ("data", None)] {
            assert!(!is_navigation_allowed(s, h, "", None, None), "{s} {h:?}");
        }
    }

    #[test]
    fn a_redirect_hop_to_another_host_is_refused_even_when_the_first_url_was_fine() {
        // Tauri asks once per hop: the first URL is the app, the next is not.
        let hops = [("https", Some("evolvepreneuriq.app")), ("https", Some("evil.example"))];
        let allowed: Vec<bool> = hops.iter().map(|(s, h)| is_navigation_allowed(s, *h, "", None, None)).collect();
        assert_eq!(allowed, vec![true, false]);
    }

    #[test]
    fn lookalike_hosts_are_not_same_site() {
        for h in ["evolvepreneuriq.app.evil.com", "evil-evolvepreneuriq.app", "xevolvepreneuriq.app", "evolvepreneuriq.app.", "EVOLVEPRENEURIQ.APP", "messager.evolvepreneuriq.app", "evolvepreneuriq.com"] {
            assert!(!is_same_site("https", Some(h), "", None, None), "{h}");
        }
        assert!(is_same_site("https", Some("evolvepreneuriq.app"), "", None, None));
    }

    #[test]
    fn an_address_with_a_user_name_or_password_is_not_same_site() {
        // https://evolvepreneuriq.app@evil.com/ parses to host evil.com; https://evil.com@evolvepreneuriq.app/ to host evolvepreneuriq.app WITH a user name: both refused.
        assert!(!is_same_site("https", Some("evil.com"), "evolvepreneuriq.app", None, None));
        assert!(!is_same_site("https", Some("evolvepreneuriq.app"), "evil.com", None, None));
        assert!(!is_same_site("https", Some("evolvepreneuriq.app"), "", Some("secret"), None));
        assert!(!is_navigation_allowed("https", Some("evolvepreneuriq.app"), "evil.com", None, None));
    }

    #[test]
    fn only_http_s_is_same_site() {
        for s in ["http", "mailto", "ftp", "file", "javascript", "data", "about", "evolve"] {
            assert!(!is_same_site(s, Some("evolvepreneuriq.app"), "", None, None), "{s}");
        }
    }

    #[test]
    fn only_the_default_port_is_same_site() {
        assert!(is_same_site("https", Some("evolvepreneuriq.app"), "", None, None));
        assert!(is_same_site("https", Some("evolvepreneuriq.app"), "", None, Some(443)));
        for port in [80u16, 8443, 8080, 444, 1] {
            assert!(!is_same_site("https", Some("evolvepreneuriq.app"), "", None, Some(port)), "{port}");
        }
    }

    #[test]
    fn a_refused_navigation_goes_home_on_our_host_and_is_dropped_everywhere_else() {
        // another site, a mailto, a script: never opened from a redirect (Auditor minor 1), only dropped
        assert_eq!(refused_action(Some("accounts.example.com")), Refusal::Nothing);
        assert_eq!(refused_action(Some("example.com")), Refusal::Nothing);
        assert_eq!(refused_action(None), Refusal::Nothing);
        assert_eq!(refused_action(Some("evolvepreneuriq.app")), Refusal::GoHome);
        assert_eq!(refused_action(Some("evolvepreneuriq.app.evil.com")), Refusal::Nothing);
    }

    #[test]
    fn a_mailto_cannot_attach_a_local_file() {
        for bad in ["mailto:a@b.test?attach=/etc/passwd", "mailto:a@b.test?subject=x&attachment=C:\\x", "MAILTO:a@b.test?ATTACH=/x", "mailto:a@b.test?subject=x;attach=/x", "mailto:a@b.test?%61ttach=/x", "mailto:a@b.test?attAch=/x"] {
            assert!(!is_openable_external(bad), "{bad}");
        }
        assert!(is_openable_external("mailto:a@b.test?subject=attach"), "the word in a value is fine");
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
