//! Evolve Messenger 1.0.1 shell wiring: the start address, the host rule, navigation and downloads, the 401 pause, the bigger window, microphone permission.
//! std-only, like tests/acl.rs.
use std::fs;

fn main_rs() -> String {
    fs::read_to_string("src/main.rs").unwrap()
}

#[test]
fn the_app_opens_the_start_address_and_the_old_phone_host_is_gone_from_the_code() {
    let src = main_rs();
    assert!(src.contains("tauri::Url::parse(&start::start_url())"));
    assert!(!src.contains("const APP_URL"));
    for f in ["src/main.rs", "src/guard.rs", "src/start.rs", "capabilities/default.json"] {
        let t = fs::read_to_string(f).unwrap();
        let live: Vec<&str> = t.lines().filter(|l| l.contains("messager.evolvepreneuriq.app") && !l.trim_start().starts_with("//") && !l.contains("(\"https\", Some(")).collect();
        assert!(live.is_empty(), "{f} still names the phone host: {live:?}");
    }
}

#[test]
fn the_host_in_the_page_scripts_comes_from_the_one_constant() {
    let src = main_rs();
    assert!(src.contains("location.hostname !== '__HOST__'") && src.contains("u.hostname === '__HOST__'"));
    assert_eq!(src.matches(".replace(\"__HOST__\", guard::APP_HOST)").count(), 3, "both scripts on page load and the timer");
    let guard = fs::read_to_string("src/guard.rs").unwrap();
    let start = fs::read_to_string("src/start.rs").unwrap();
    let h = |t: &str| t.lines().find(|l| l.contains("pub const APP_HOST")).unwrap().split('"').nth(1).unwrap().to_string();
    assert_eq!(h(&guard), h(&start), "start.rs and guard.rs name the same host");
}

#[test]
fn the_window_user_agent_is_a_browser_string_with_the_app_suffix() {
    let src = main_rs();
    assert!(src.contains(".user_agent(&start::user_agent(std::env::consts::OS, env!(\"CARGO_PKG_VERSION\")))"));
    assert!(!src.contains("Tauri/2"), "the old replacement agent is gone");
}

#[test]
fn links_to_other_pages_of_the_site_open_in_the_system_browser_not_in_the_chat_window() {
    let src = main_rs();
    assert!(src.contains("var internal = u.hostname === '__HOST__' && /^\\/(__PATHS__)([\\/?#]|$)/.test(u.pathname);"));
    assert!(src.contains(".replace(\"__PATHS__\", &guard::IN_WINDOW.join(\"|\"))"), "the allowed first path parts come from guard.rs, one list");
}

#[test]
fn the_window_is_desktop_sized_stays_on_its_host_and_saves_downloads_safely() {
    let src = main_rs();
    assert!(src.contains(".inner_size(1100.0, 760.0)") && src.contains(".min_inner_size(800.0, 520.0)"));
    assert!(src.contains(".on_navigation(|url| guard::is_navigation_allowed(url.scheme(), url.host_str()))"));
    let d = src.find(".on_download(").expect("download handler");
    let body = &src[d..d + 900];
    assert!(body.contains("guard::safe_file_name("), "the name is made safe");
    assert!(body.contains("download_dir()"), "files go to the Downloads folder");
    assert!(body.contains("*destination = dir.join(name);"), "the safe name, not the raw one, is what is saved");
}

#[test]
fn a_401_pauses_the_timer_and_a_count_resumes_it() {
    let src = main_rs();
    assert!(src.contains("if (r.status === 401) { window.__TAURI_INTERNALS__.invoke('report_signed_out'); return; }"));
    let c = src.find("async fn report_signed_out").unwrap();
    assert!(src[c..c + 300].contains("from_app(&webview)"), "origin guard");
    let t = src.find("if SIGNED_OUT.load(").expect("timer checks the flag");
    assert!(src[t..t + 200].contains("continue;"));
    let u = src.find("async fn report_unread").unwrap();
    assert!(src[u..u + 600].contains("SIGNED_OUT.store(false"), "a count clears the pause");
}

#[test]
fn the_voice_note_microphone_is_declared_for_macos() {
    let info = fs::read_to_string("Info.plist").unwrap();
    assert!(info.contains("<key>NSMicrophoneUsageDescription</key>"));
    let ent = fs::read_to_string("Entitlements.plist").unwrap();
    let i = ent.find("com.apple.security.device.audio-input").expect("audio-input");
    assert!(ent[i..].contains("<true/>"));
}
