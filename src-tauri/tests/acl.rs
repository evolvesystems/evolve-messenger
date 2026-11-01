//! Every command in generate_handler! must be listed in build.rs AND granted in the capability, and nothing may be granted that the app does not have.
//! A page that calls an ungranted command fails with "Command X not allowed by ACL" (EvolveApp 2.4.21). std-only, so it also runs with `rustc --test`.
use std::fs;

fn handler_commands() -> Vec<String> {
    let src = fs::read_to_string("src/main.rs").unwrap();
    let start = src.find("generate_handler![").expect("generate_handler! not found") + "generate_handler![".len();
    let body = &src[start..start + src[start..].find(']').unwrap()];
    body.split(',').map(|s| s.trim().rsplit("::").next().unwrap().to_string()).filter(|s| !s.is_empty()).collect()
}

fn permission(cmd: &str) -> String {
    format!("allow-{}", cmd.replace('_', "-"))
}

#[test]
fn there_is_at_least_one_command_and_the_parser_found_them() {
    assert!(handler_commands().len() >= 2, "{:?}", handler_commands());
}

#[test]
fn every_handler_command_is_listed_in_the_app_manifest() {
    let build = fs::read_to_string("build.rs").unwrap();
    assert!(build.contains("AppManifest"), "build.rs has no AppManifest");
    for c in handler_commands() {
        assert!(build.contains(&format!("\"{c}\"")), "{c} missing from build.rs");
    }
}

#[test]
fn every_handler_command_is_granted_in_the_capability() {
    let cap = fs::read_to_string("capabilities/default.json").unwrap();
    for c in handler_commands() {
        assert!(cap.contains(&format!("\"{}\"", permission(&c))), "{c}: capability lacks {}", permission(&c));
    }
}

#[test]
fn nothing_is_granted_that_the_app_does_not_have() {
    let cap = fs::read_to_string("capabilities/default.json").unwrap();
    let have: Vec<String> = handler_commands().iter().map(|c| permission(c)).collect();
    for line in cap.lines() {
        let t = line.trim().trim_end_matches(',').trim_matches('"');
        if t.starts_with("allow-") {
            assert!(have.contains(&t.to_string()), "{t} is granted but there is no such command");
        }
    }
}

#[test]
fn only_the_messenger_host_is_a_remote_url() {
    let cap = fs::read_to_string("capabilities/default.json").unwrap();
    assert!(cap.contains("https://messager.evolvepreneuriq.app/*"));
    assert!(!cap.contains("*.evolvepreneuriq.app"), "no wildcard subdomains: tenant pages live there");
    assert!(!cap.contains("\"https://evolvepreneuriq.app/*\""));
}
