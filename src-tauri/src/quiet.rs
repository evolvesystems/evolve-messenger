//! When may an update install by itself (Messenger 1.0.2, docs/specs/desktop/DESKTOP_AUTO_INSTALL.md in EIQ)? Never while the window is in front. std-only, tested with `rustc --test src/quiet.rs`.

/// The window must have been hidden or out of focus for this long.
pub const QUIET_AFTER_SECS: u64 = 600;
/// And the app must have been running at least this long (a fresh start is not a quiet moment).
pub const MIN_UPTIME_SECS: u64 = 300;

/// `away_since`: when the window last left the front (None = it is in front now). `downloads`: files being saved right now.
pub fn is_quiet(away_since: Option<u64>, now: u64, downloads: u32, started_at: u64) -> bool {
    let Some(since) = away_since else { return false };
    downloads == 0 && now.saturating_sub(since) >= QUIET_AFTER_SECS && now.saturating_sub(started_at) >= MIN_UPTIME_SECS
}

/// The setting file holds "on" or "off". Anything else, or no file, means ON (the default).
pub fn auto_install_enabled(file_text: Option<&str>) -> bool {
    !matches!(file_text.map(|t| t.trim().to_ascii_lowercase()).as_deref(), Some("off"))
}

pub fn setting_text(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

/// The one-line marker written before the restart; the next start shows the notice and deletes it.
pub fn marker_text(version: &str) -> String {
    format!("updated to {}", version.trim())
}

/// A version from a marker file, only if it is a plain version number (the file is local but is never trusted into a notification as is).
pub fn version_from_marker(text: &str) -> Option<String> {
    let v = text.trim().strip_prefix("updated to ")?.trim();
    let ok = !v.is_empty() && v.len() <= 20 && v.split('.').count() == 3 && v.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    ok.then(|| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_quiet_while_the_window_is_in_front() {
        assert!(!is_quiet(None, 1_000_000, 0, 0));
    }

    #[test]
    fn quiet_only_after_ten_minutes_away_with_no_download_and_enough_uptime() {
        let started = 1000;
        assert!(!is_quiet(Some(5000), 5000 + QUIET_AFTER_SECS - 1, 0, started), "too soon");
        assert!(is_quiet(Some(5000), 5000 + QUIET_AFTER_SECS, 0, started));
        assert!(!is_quiet(Some(5000), 6000, 1, started), "a download is running");
        // away for 10 minutes but the app itself only started 100 s ago
        assert!(!is_quiet(Some(5000), 5000 + QUIET_AFTER_SECS, 0, 5000 + QUIET_AFTER_SECS - 100), "too little uptime");
        assert!(is_quiet(Some(5000), 5000 + QUIET_AFTER_SECS, 0, 5000 + QUIET_AFTER_SECS - MIN_UPTIME_SECS), "exactly enough uptime");
    }

    #[test]
    fn a_clock_that_goes_backwards_is_not_quiet_and_does_not_underflow() {
        assert!(!is_quiet(Some(10_000), 5, 0, 0));
    }

    #[test]
    fn the_setting_defaults_to_on_and_only_off_turns_it_off() {
        assert!(auto_install_enabled(None));
        assert!(auto_install_enabled(Some("on")));
        assert!(auto_install_enabled(Some("garbage")));
        assert!(auto_install_enabled(Some("")));
        assert!(!auto_install_enabled(Some("off")));
        assert!(!auto_install_enabled(Some(" OFF\n")));
        assert_eq!(setting_text(true), "on");
        assert_eq!(setting_text(false), "off");
    }

    #[test]
    fn the_marker_round_trips_and_junk_is_refused() {
        assert_eq!(version_from_marker(&marker_text("1.0.2")), Some("1.0.2".to_string()));
        for bad in ["", "updated to ", "updated to 1.0", "updated to 1.0.x", "updated to <b>1.0.2</b>", "hello", "updated to 1.0.2.3", "updated to 99999999999999999999.0.0"] {
            assert_eq!(version_from_marker(bad), None, "{bad:?}");
        }
    }
}
