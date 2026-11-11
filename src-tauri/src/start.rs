//! The page the desktop app opens (Evolve Messenger 1.0.1): the new desktop Team Chat page on the main host (docs/specs/desktop/DESKTOP_TEAM_CHAT_PAGE.md in EIQ).
//! One place for the address. std-only, tested with plain `rustc --test src/start.rs`.
//!
//! RELEASE SAFETY: START_PATH is a placeholder until the page is live and its path is confirmed. `CONFIRMED` flips to true in the same commit that sets the real path;
//! tests/release_ready.rs (run when Publish is on) refuses to publish while it is false.

pub const APP_HOST: &str = "evolvepreneuriq.app";
pub const START_PATH: &str = "/chat/desktop";
pub const CONFIRMED: bool = true;

pub fn start_url() -> String {
    format!("https://{APP_HOST}{START_PATH}")
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
        assert!(start_url().starts_with("https://evolvepreneuriq.app/"));
        assert!(is_valid_path(START_PATH));
    }

    #[test]
    fn bad_paths_are_refused() {
        for bad in ["", "chat", "//evil.test/x", "/a/../b", "/a?b=1", "/a#b", "/a b", "/a\nb", "/a\\b", "https://evil.test/"] {
            assert!(!is_valid_path(bad), "{bad:?}");
        }
        assert!(!is_valid_path(&format!("/{}", "a".repeat(250))));
    }
}
