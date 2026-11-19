//! Evolve Messenger 1.0.1 shell wiring: the start address, the host rule, navigation and downloads, the 401 pause, the bigger window, microphone permission.
//! std-only, like tests/acl.rs.
use std::fs;

fn main_rs() -> String {
    fs::read_to_string("src/main.rs").unwrap()
}

#[test]
fn the_app_opens_the_desktop_team_chat_page_on_the_main_host() {
    let src = main_rs();
    assert!(src.contains("tauri::Url::parse(&start::start_url())"));
    assert!(!src.contains("const APP_URL"));
    for f in ["src/start.rs", "capabilities/default.json"] {
        let t = fs::read_to_string(f).unwrap();
        assert!(!t.contains("messager.evolvepreneuriq"), "{f} must not name the phone host");
    }
    let start = fs::read_to_string("src/start.rs").unwrap();
    assert!(start.contains("pub const START_PATH: &str = \"/chat/desktop\";"), "the start is the desktop Team Chat page");
    assert!(start.contains("pub const APP_HOST: &str = \"evolvepreneuriq.app\";"));
}

#[test]
fn the_host_in_the_page_scripts_comes_from_the_one_constant() {
    let src = main_rs();
    let links = fs::read_to_string("src/links.js").unwrap();
    assert!(src.contains("location.hostname !== '__HOST__'") && links.contains("u.hostname === '__HOST__'"));
    assert_eq!(src.matches(".replace(\"__HOST__\", guard::APP_HOST)").count(), 6, "scripts on page load (unread, live stream, back bar), the timer, and the window coming to the front");
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
fn every_page_of_the_site_stays_in_the_window_and_other_sites_open_in_the_system_browser() {
    let src = main_rs();
    let links = fs::read_to_string("src/links.js").unwrap();
    assert!(src.contains("const EXTERNAL_LINKS_JS: &str = include_str!(\"links.js\");"), "the link script is its own file, run by the node harness");
    assert!(links.contains("return u.protocol === 'https:' && u.hostname === '__HOST__' && !u.username && !u.password && (u.port === '' || u.port === '443');"), "the same-site rule: https, exact host, no userinfo, default port");
    assert!(links.contains("if (!sameSite(u) || el.target === '_blank') {"), "anything not same-site, and target=_blank, goes to the system browser");
    assert!(!src.contains("IN_WINDOW") && !src.contains("__PATHS__") && !links.contains("__PATHS__"), "no first-path-word list any more");
    assert!(!fs::read_to_string("src/guard.rs").unwrap().contains("is_in_window_path"));
}

#[test]
fn back_forward_and_home_move_around_the_window_and_the_tray_has_home() {
    let src = main_rs();
    let links = fs::read_to_string("src/links.js").unwrap();
    assert!(links.contains("e.key === 'ArrowLeft'") && links.contains("history.back();"), "Alt+Left goes back");
    assert!(links.contains("e.key === 'ArrowRight'") && links.contains("history.forward();"));
    assert!(links.contains("e.button === 3") && links.contains("e.button === 4"), "mouse back and forward buttons");
    assert!(links.contains("e.key === 'Home'") && links.contains("location.assign('__START__')"), "Alt+Home goes to the chat");
    assert!(src.contains(".replace(\"__START__\", start::START_PATH)"), "Home is the one start path from start.rs");
    assert!(src.contains("\"home\" => {") && src.contains("MenuItem::with_id(app, \"home\", \"Home (chat)\", true, None::<&str>)?"), "the tray has Home");
    assert!(src.contains("fn go_home(app: &tauri::AppHandle)") && src.contains("w.navigate(u)"), "Home navigates the webview itself");
    assert!(src.contains("show_window(app);\n                        go_home(app);"), "the tray Home calls go_home, not a page script");
    assert!(!src.contains("location.assign('{}')"), "no page-script navigation for the tray");
    assert!(src.contains("&[&m_open, &m_home, &m_login,"));
}

#[test]
fn the_window_is_desktop_sized_stays_on_its_host_and_saves_downloads_safely() {
    let src = main_rs();
    assert!(src.contains(".inner_size(1100.0, 760.0)") && src.contains(".min_inner_size(1024.0, 600.0)"), "the two-column layout starts at 900 px, the window never gets narrower than 1024");
    assert!(src.contains(".on_navigation({") && src.contains("guard::is_navigation_allowed(url.scheme(), url.host_str(), url.username(), url.password(), url.port())"), "every navigation hop is checked with the user name, password and port");
    assert!(src.contains("guard::refused_action(url.host_str(), url.as_str())") && src.contains("guard::Refusal::OpenInBrowser =>") && src.contains("guard::Refusal::GoHome => go_home(&h2)"), "a refused hop opens the browser or goes home, never a blank window");
    let flat: String = src.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(flat.contains("guard::Refusal::OpenInBrowser=>{usetauri_plugin_shell::ShellExt;let_=h2.shell().open(target,None);}"), "a refused hop to another site really opens the system browser");
    assert!(flat.contains("guard::Refusal::Nothing=>{}});false}})"), "the refused navigation itself is cancelled (the closure returns false after handling it)");
    assert!(src.contains(".on_new_window({") && src.contains("tauri::webview::NewWindowResponse::Deny") && src.contains("guard::is_openable_external(url.as_str())"), "window.open, middle click and target=_blank go to the system browser");
    let d = src.find(".on_download(").expect("download handler");
    let body = &src[d..d + 900];
    assert!(body.contains("guard::safe_file_name("), "the name is made safe");
    assert!(body.contains("download_dir()"), "files go to the Downloads folder");
    assert!(body.contains("*destination = dir.join(name);"), "the safe name, not the raw one, is what is saved");
}

#[test]
fn a_401_pauses_the_timer_and_a_count_resumes_it() {
    let src = main_rs();
    assert!(src.contains("if (r.status === 401 || r.status === 403) { window.__TAURI_INTERNALS__.invoke('report_signed_out'); return; }"));
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
    assert!(src.contains("WindowEvent::Focused(true) => {") && src.contains("AWAY_SINCE.store(0,"), "in front resets the clock");
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

#[test]
fn the_unread_number_comes_from_the_messenger_badge_endpoint_total() {
    let src = main_rs();
    assert!(src.contains("fetch('/messenger/api/badge', {credentials: 'include', signal: ctl.signal, headers:"));
    assert!(src.contains("parseInt(d.total, 10)"));
    assert!(!src.contains("check-notifications"), "the EvolveApp endpoint is gone from the Messenger shell");
}

#[test]
fn nothing_in_the_shell_reloads_or_navigates_on_a_resize() {
    let src = main_rs();
    assert!(!src.contains("WindowEvent::Resized"), "no resize handler");
    assert!(!src.contains("location.reload") && !src.contains("location.href") && !src.contains("location.replace"), "no scripted reload anywhere");
    // 1.0.8: the ONE explicit navigation is go_home (tray Home, a refused address on our own host), never from a resize or a timer.
    assert_eq!(src.matches(".navigate(").count(), 1, "exactly one navigate call");
    let g = src.find("fn go_home(").expect("go_home");
    let end = g + src[g..].find("\n}\n").unwrap();
    assert!(src[g..end].contains(".navigate("), "and it is inside go_home");
}

#[test]
fn the_unread_poll_never_runs_on_a_sign_in_page_and_asks_for_json_as_an_ajax_call() {
    let src = main_rs();
    let poll = &src[src.find("const UNREAD_POLL_JS").unwrap()..src.find("const EXTERNAL_LINKS_JS").unwrap()];
    assert!(poll.contains("login|login_check|2fa|forgot|reset|register|logout"), "sign-in pages are skipped");
    assert!(poll.find("login|login_check").unwrap() < poll.find("fetch(").unwrap(), "the skip comes before the request");
    assert!(poll.contains("'X-Requested-With': 'XMLHttpRequest'") && poll.contains("'Accept': 'application/json'"), "the site must treat it as an ajax call, never save it as the post-login page");
}

#[test]
fn the_unread_poll_is_every_15_seconds_one_call_at_a_time_and_runs_when_the_window_comes_to_the_front() {
    let src = main_rs();
    assert!(src.contains("const UNREAD_POLL_SECS: u64 = 15;"));
    assert!(src.contains("std::thread::sleep(Duration::from_secs(UNREAD_POLL_SECS));"), "the timer uses the constant");
    let poll = &src[src.find("const UNREAD_POLL_JS").unwrap()..src.find("const EXTERNAL_LINKS_JS").unwrap()];
    assert!(poll.contains("window.__evolve_poll_busy") && poll.contains("window.__evolve_poll_busy = false"), "no second call while one is in flight, and the flag is always released");
    assert!(poll.contains("finally"), "the release is in a finally block");
    assert!(poll.contains("ctl.abort()") && poll.contains("10000"), "a call that never answers is cut after 10 s");
    assert!(poll.contains("took * 4"), "a slow answer slows the next calls");
    let focus = &src[src.find("WindowEvent::Focused(true)").unwrap()..];
    assert!(focus[..400].contains("UNREAD_POLL_JS"), "coming to the front polls at once");
}

#[test]
fn the_live_unread_stream_is_started_per_page_and_the_poll_stays_as_the_backstop() {
    let src = main_rs();
    assert!(src.contains("const LIVE_UNREAD_JS: &str = include_str!(\"live_unread.js\");"));
    assert!(src.contains("webview.eval(&LIVE_UNREAD_JS.replace(\"__HOST__\", guard::APP_HOST))"), "started on every app page load");
    let poll = &src[src.find("const UNREAD_POLL_JS").unwrap()..src.find("const LIVE_UNREAD_JS").unwrap()];
    assert!(poll.contains("window.__evolve_live_ok && Date.now() - (window.__evolve_last_poll || 0) < 60000"), "only a 60 s backstop while the stream is open");
    assert!(poll.contains("window.__evolve_last_poll = Date.now();"));
    let js = fs::read_to_string("src/live_unread.js").unwrap();
    assert!(js.contains("/api/v1/desktop/live-auth") && js.contains("'Authorization': 'Bearer '"), "key from the site, bearer header (never the token in the URL)");
    assert!(!js.contains("authorization="), "no token in a URL");
    assert!(fs::metadata("tests/live_unread_harness.mjs").is_ok(), "the node harness that tests the script logic ships with it");
}

#[test]
fn profile_and_notification_pages_stay_in_the_window_with_a_back_to_chat_bar() {
    let src = main_rs();
    assert!(src.contains("const BACK_TO_CHAT_JS: &str = include_str!(\"back_to_chat.js\");"));
    assert!(src.contains("webview.eval(&BACK_TO_CHAT_JS.replace(\"__HOST__\", guard::APP_HOST))"), "added on every app page load");
    let guard = fs::read_to_string("src/guard.rs").unwrap();
    assert!(guard.contains("pub fn is_same_site("), "every page of the site, Profile and Notification settings included, is in the window (1.0.8)");
    let js = fs::read_to_string("src/back_to_chat.js").unwrap();
    assert!(js.contains("bar.href = '/chat/desktop';"), "the bar leads back to the desktop chat page");
    assert!(fs::metadata("tests/back_to_chat_harness.mjs").is_ok(), "the node harness for the script ships with it");
}

#[test]
fn the_node_harnesses_for_the_page_scripts_pass() {
    // links.js (same-site rule, back / forward / Home) and back_to_chat.js run in a fake page under node; skipped where node is not installed.
    if std::process::Command::new("node").arg("--version").output().is_err() {
        return;
    }
    for (harness, script) in [("tests/links_harness.mjs", "src/links.js"), ("tests/back_to_chat_harness.mjs", "src/back_to_chat.js")] {
        let out = std::process::Command::new("node").arg(harness).arg(script).output().expect("node runs");
        assert!(out.status.success(), "{harness}: {}", String::from_utf8_lossy(&out.stdout));
    }
    assert!(fs::metadata("tests/links_harness.mjs").is_ok());
}
