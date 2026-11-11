//! Checks that must pass before a release is PUBLISHED. Ignored in normal runs (the key does not exist until John creates it); the release workflow runs them with
//! `cargo test --test release_ready -- --include-ignored` when publish is true.
use std::fs;

#[test]
#[ignore = "needs the Messenger updater public key in tauri.conf.json"]
fn the_updater_public_key_is_set_and_is_a_public_key() {
    let t = fs::read_to_string("tauri.conf.json").unwrap();
    let after = &t[t.find("\"pubkey\"").expect("pubkey in tauri.conf.json")..];
    let key = after.split('"').nth(3).unwrap().trim();
    assert_ne!(key, "REPLACE_WITH_MESSENGER_UPDATER_PUBLIC_KEY", "set the public key (see the README)");
    assert!(key.len() > 40, "too short to be a minisign public key");
    assert!(key.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='), "must be the base64 text");
    assert!(!key.to_lowercase().contains("secret"), "that looks like a private key");
}

#[test]
#[ignore = "the desktop Team Chat page path is not confirmed yet (src/start.rs CONFIRMED)"]
fn the_start_page_is_confirmed_before_publishing() {
    let t = fs::read_to_string("src/start.rs").unwrap();
    assert!(t.contains("pub const CONFIRMED: bool = true;"), "set the real START_PATH and flip CONFIRMED once the desktop Team Chat page is live");
}
