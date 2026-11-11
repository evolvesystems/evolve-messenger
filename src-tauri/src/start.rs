//! The page the desktop app opens (Evolve Messenger 1.0.1): the new desktop Team Chat page on the main host (docs/specs/desktop/DESKTOP_TEAM_CHAT_PAGE.md in EIQ).
//! One place for the address. std-only, tested with plain `rustc --test src/start.rs`.
//!
//! RELEASE SAFETY: START_PATH is a placeholder until the page is live and its path is confirmed. `CONFIRMED` flips to true in the same commit that sets the real path;
//! tests/release_ready.rs (run when Publish is on) refuses to publish while it is false.

// 1.0.2 (task 5828, John 11 Nov): the app loads the PHONE Messenger host with MOBILE APPS' two-column layout. On that host the bare root is the app's own entry (the host router sends / to the app shell).
pub const APP_HOST: &str = "messager.evolvepreneuriq.app";
pub const START_PATH: &str = "/";
/// The screen the app opens (MOBILE APPS, 11 Nov): Team Chat. Plain "/" signed in lands on the Inbox. A fixed query, validated like the path.
pub const START_QUERY: &str = "screen=team-chat";
pub const CONFIRMED: bool = true;

pub fn start_url() -> String {
    format!("https://{APP_HOST}{START_PATH}?{START_QUERY}")
}

/// A start query is plain `name=value` pairs joined by `&`: letters, digits, `-`, `_`, `=`, `&` only.
pub fn is_valid_query(q: &str) -> bool {
    !q.is_empty() && q.len() <= 100 && q.chars().all(|c| c.is_ascii_alphanumeric() || "-_=&".contains(c))
}

/// The window's User-Agent: a normal browser string for the platform with " EvolveMessenger/<version>" appended, so the site still sees an ordinary Chrome or WebKit and the
/// desktop chat page can recognise the app by the suffix (UI Sweep, commit 6db4382ebc). The base is fixed per platform because the webview's own default cannot be read before the window exists.
pub fn user_agent(os: &str, version: &str) -> String {
    let base = match os {
        "windows" => "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0",
        "macos" => "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15",
        _ => "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15",
    };
    format!("{base} EvolveMessenger/{version}")
}

/// A start path is an absolute path on the host: no scheme, no host, no whitespace, no query or fragment, no `..`.
pub fn is_valid_path(p: &str) -> bool {
    p.starts_with('/') && !p.starts_with("//") && p.len() <= 200 && !p.contains("..") && !p.contains('?') && !p.contains('#') && !p.chars().any(|c| c.is_whitespace() || c.is_control() || c == '\\')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_url_is_https_on_the_main_host_and_the_path_is_valid() {
        assert!(start_url().starts_with("https://messager.evolvepreneuriq.app/"));
        assert!(is_valid_path(START_PATH));
        assert!(is_valid_query(START_QUERY));
        assert_eq!(start_url(), "https://messager.evolvepreneuriq.app/?screen=team-chat");
    }

    #[test]
    fn bad_queries_are_refused() {
        for bad in ["", "a=b c", "a=b#c", "a=<b>", "a=b?c", "javascript:x", "a=b\n"] {
            assert!(!is_valid_query(bad), "{bad:?}");
        }
        assert!(!is_valid_query(&"a".repeat(101)));
    }

    #[test]
    fn the_user_agent_is_a_normal_browser_with_the_app_suffix_appended() {
        for os in ["windows", "macos", "linux", "freebsd"] {
            let ua = user_agent(os, "1.0.1");
            assert!(ua.starts_with("Mozilla/5.0 ("), "{os}: {ua}");
            assert!(ua.contains("AppleWebKit/"), "{os}");
            assert!(ua.ends_with(" EvolveMessenger/1.0.1"), "{os}: the suffix comes last");
        }
        assert!(user_agent("windows", "1.0.1").contains("Chrome/"));
        assert!(user_agent("macos", "2.0.0").ends_with("EvolveMessenger/2.0.0"));
    }

    #[test]
    fn bad_paths_are_refused() {
        for bad in ["", "chat", "//evil.test/x", "/a/../b", "/a?b=1", "/a#b", "/a b", "/a\nb", "/a\\b", "https://evil.test/"] {
            assert!(!is_valid_path(bad), "{bad:?}");
        }
        assert!(!is_valid_path(&format!("/{}", "a".repeat(250))));
    }
}
