//! Turns a failed in-app update into a kind the dialog can explain and the log can record.
//! std-only on purpose: it is tested with plain `rustc --test src/update_error.rs` where the Tauri toolchain is missing.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Fetching the update file from GitHub.
    Download,
    /// Checking the signature and replacing the installed app.
    Install,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Network,
    Signature,
    /// The app could not replace itself (read-only volume, no permission, running from a disk image).
    Install,
    /// The pending update was already used by an earlier click. It is not put back on failure: "Try Again" re-runs the
    /// update check, which stores a fresh one.
    Consumed,
    Unknown,
}

impl Kind {
    pub fn code(self) -> &'static str {
        match self {
            Kind::Network => "network",
            Kind::Signature => "signature",
            Kind::Install => "install",
            Kind::Consumed => "consumed",
            Kind::Unknown => "unknown",
        }
    }
}

pub fn classify(stage: Stage, message: &str) -> Kind {
    let m = message.to_lowercase();
    if m.contains("already consumed") || m.contains("no pending update") {
        return Kind::Consumed;
    }
    if m.contains("signature") || m.contains("minisign") || m.contains("public key") {
        return Kind::Signature;
    }
    if stage == Stage::Install {
        return Kind::Install;
    }
    let network = ["request", "network", "dns", "connect", "timed out", "timeout", "http", "tls", "ssl", "certificate", "proxy", "error sending"];
    if network.iter().any(|w| m.contains(w)) {
        return Kind::Network;
    }
    if stage == Stage::Download {
        return Kind::Network;
    }
    Kind::Unknown
}

/// What the JS catch receives: "<kind>|<detail>". The detail is trimmed to one short line; it is the error text of the
/// updater (no URLs with tokens: the update URL is a public GitHub release URL), never page content.
pub fn encode(kind: Kind, message: &str) -> String {
    let one_line: String = message.chars().map(|c| if c.is_control() { ' ' } else { c }).take(300).collect();
    format!("{}|{}", kind.code(), one_line.trim())
}

/// The sentence shown to the person (notification) for a failed update. No error text, no URLs.
pub fn user_text(kind: Kind) -> &'static str {
    match kind {
        Kind::Network => "Could not download the update. Check your internet connection and try again from the tray menu.",
        Kind::Signature => "The update failed its security check and was not installed. Download Evolve Messenger again from evolvepreneuriq.app/download.",
        Kind::Install => "The update downloaded but could not be installed. Make sure Evolve Messenger is in your Applications / Programs folder, then try again.",
        Kind::Consumed => "That update was already used. Check for updates again from the tray menu.",
        Kind::Unknown => "The update failed. Details are in the log (tray menu, Open log folder).",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_download_is_a_network_error_unless_it_says_signature() {
        assert_eq!(classify(Stage::Download, "error sending request for url (https://github.com/...)"), Kind::Network);
        assert_eq!(classify(Stage::Download, "something odd"), Kind::Network);
        assert_eq!(classify(Stage::Download, "the signature could not be verified"), Kind::Signature);
    }

    #[test]
    fn a_failed_install_is_an_install_error_unless_it_is_the_signature() {
        assert_eq!(classify(Stage::Install, "Read-only file system (os error 30)"), Kind::Install);
        assert_eq!(classify(Stage::Install, "Permission denied (os error 13)"), Kind::Install);
        assert_eq!(classify(Stage::Install, "minisign verification failed"), Kind::Signature);
    }

    #[test]
    fn the_consumed_cases_are_recognised_at_any_stage() {
        assert_eq!(classify(Stage::Download, "Update already consumed"), Kind::Consumed);
        assert_eq!(classify(Stage::Install, "No pending update"), Kind::Consumed);
    }

    #[test]
    fn a_download_failure_without_network_words_is_still_called_network_so_the_dialog_always_has_a_hint() {
        // Known limit: a disk or permission failure while saving the file is reported as a download problem.
        assert_eq!(classify(Stage::Download, "No such file or directory (os error 2)"), Kind::Network);
        assert_eq!(classify(Stage::Download, ""), Kind::Network);
    }

    #[test]
    fn matching_is_case_insensitive_and_the_signature_wins_over_network_words() {
        assert_eq!(classify(Stage::Download, "HTTP 200 but SIGNATURE mismatch"), Kind::Signature);
        assert_eq!(classify(Stage::Install, "Update Already Consumed"), Kind::Consumed);
    }

    #[test]
    fn every_kind_has_a_distinct_code_the_dialog_can_key_on() {
        let codes = [Kind::Network, Kind::Signature, Kind::Install, Kind::Consumed, Kind::Unknown].map(Kind::code);
        for (i, a) in codes.iter().enumerate() {
            for b in &codes[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn a_pipe_or_multibyte_text_in_the_message_cannot_break_the_kind_prefix() {
        let s = encode(Kind::Network, "a|b — ü 日本語");
        assert!(s.starts_with("network|"));
        assert_eq!(s.split('|').next(), Some("network"));
        let long = encode(Kind::Install, &"日".repeat(500));
        assert_eq!(long.chars().count(), "install|".len() + 300);
    }

    #[test]
    fn the_encoded_error_is_kind_pipe_one_short_line() {
        let s = encode(Kind::Install, "line one\nline two\t\u{7}end");
        assert!(s.starts_with("install|"));
        assert!(!s.contains('\n') && !s.contains('\t'));
        assert!(encode(Kind::Network, &"x".repeat(1000)).len() <= "network|".len() + 300);
    }

    #[test]
    fn every_kind_has_a_plain_sentence_with_no_error_text_or_url_in_it() {
        for k in [Kind::Network, Kind::Signature, Kind::Install, Kind::Consumed, Kind::Unknown] {
            let t = user_text(k);
            assert!(t.len() > 20, "{:?}", k);
            assert!(!t.contains("http://") && !t.contains("error sending"), "{:?}", k);
        }
        assert_ne!(user_text(Kind::Network), user_text(Kind::Install));
    }
}
