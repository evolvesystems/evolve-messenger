//! Evolve Messenger 1.0.1 shell wiring: the start address, the host rule, navigation and downloads, the 401 pause, the bigger window, microphone permission.
//! std-only, like tests/acl.rs.
use std::fs;

fn main_rs() -> String {
    fs::read_to_string("src/main.rs").unwrap()
}

#[test]
fn the_app_opens_the_start_address_on_the_messenger_host() {
    let src = main_rs();
    assert!(src.contains("tauri::Url::parse(&start::start_url())"));
    assert!(!src.contains("const APP_URL"));
    for f in ["src/guard.rs", "src/start.rs", "capabilities/default.json"] {
        let t = fs::read_to_string(f).unwrap();
        assert!(t.contains("messager.evolvepreneuriq.app"), "{f} names the Messenger host");
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

#[test]
fn the_unread_count_drives_the_badge_and_the_tray_tooltip() {
    let src = main_rs();
    let u = src.find("async fn report_unread").unwrap();
    let body = &src[u..u + 900];
    assert!(body.contains("show_badge(&app, count.max(0));"));
    assert!(src.contains("w.set_badge_count(badge::badge_value(count))"));
    assert!(src.contains("w.set_overlay_icon(icon)"), "Windows taskbar overlay");
}

#[test]
fn automatic_install_waits_for_a_quiet_moment_and_respects_the_tray_switch() {
    let src = main_rs();
    let t = src.find("quiet moment, installing automatically").expect("auto install");
    let before = &src[t - 700..t];
    assert!(before.contains("auto_install_on(&h)"), "the tray switch is checked");
    assert!(before.contains("quiet::is_quiet("), "the quiet rule is checked");
    assert!(before.contains("pending"), "only when an update was found");
    assert!(src[t..t + 200].contains("install_update(h2)"), "the same signed install as the tray item");
    assert!(src.contains("\"auto_install\" =>"), "tray switch handler");
    assert!(src.contains("\"Install updates automatically\""));
}

#[test]
fn the_window_being_in_front_or_downloading_blocks_the_automatic_install() {
    let src = main_rs();
    assert!(src.contains("WindowEvent::Focused(true) => AWAY_SINCE.store(0,"), "in front resets the clock");
    assert!(src.contains("DOWNLOADS.fetch_add(1"), "a requested download is counted");
    assert!(src.contains("DOWNLOADS.load("), "and read by the install rule");
}

#[test]
fn the_updated_to_marker_is_written_before_the_restart_and_shown_once() {
    let src = main_rs();
    let m = src.find("quiet::marker_text(&update.version)").expect("marker written");
    let r = src.find("app.restart();").expect("restart");
    assert!(m < r, "marker before restart");
    assert!(src.contains("std::fs::remove_file(&p)"), "the marker is deleted after it is read");
    assert!(src.contains("quiet::version_from_marker(&text)"), "only a plain version number is shown");
}
