//! The version lives in tauri.conf.json and Cargo.toml and they must be identical (the release workflow checks the same, but a test catches it earlier).
use std::fs;

fn cargo_version() -> String {
    let t = fs::read_to_string("Cargo.toml").unwrap();
    let line = t.lines().find(|l| l.starts_with("version")).expect("version in Cargo.toml");
    line.split('"').nth(1).unwrap().to_string()
}

fn conf_version() -> String {
    let t = fs::read_to_string("tauri.conf.json").unwrap();
    let after = &t[t.find("\"version\"").expect("version in tauri.conf.json")..];
    after.split('"').nth(3).unwrap().to_string()
}

#[test]
fn the_two_versions_are_identical() {
    assert_eq!(cargo_version(), conf_version());
}

#[test]
fn the_version_looks_like_a_release_version() {
    let v = cargo_version();
    let parts: Vec<&str> = v.split('.').collect();
    assert_eq!(parts.len(), 3, "{v}");
    assert!(parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())), "{v}");
}

#[test]
fn the_app_has_its_own_identity_not_evolveapps() {
    let t = fs::read_to_string("tauri.conf.json").unwrap();
    assert!(t.contains("\"identifier\": \"com.evolveapp.messenger\""));
    assert!(t.contains("\"productName\": \"Evolve Messenger\""));
    assert!(!t.contains("com.evolveapp.desktop"));
    assert!(t.contains("/api/v1/desktop/apps/messenger/updates/"), "its own updater feed");
    assert!(!t.contains("/api/v1/desktop/updates/"), "never EvolveApp's feed");
}
