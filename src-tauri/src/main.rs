// Evolve Messenger: the Evolve team chat as its own desktop app.
//
// One window that shows https://messager.evolvepreneuriq.app (the same site the phone app loads), a tray icon, a native notification when unread chats rise,
// an updater that reads /api/v1/desktop/apps/messenger/updates/... (never EvolveApp's feed), optional launch at login (off by default) and a single running copy.
// Everything testable without the Tauri toolchain lives in guard.rs, unread.rs and update_error.rs (std-only; unit tests inside, drift tests in src-tauri/tests).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use std::time::Duration;

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::PageLoadEvent,
    Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::UpdaterExt;

mod diag;
mod guard;
mod unread;
mod update_error;

const APP_URL: &str = "https://messager.evolvepreneuriq.app";
const WINDOW: &str = "main";

static UNREAD: unread::Tracker = unread::Tracker::new();

struct PendingUpdate(Mutex<Option<tauri_plugin_updater::Update>>);
struct InstallItem(MenuItem<tauri::Wry>);

/// Is the caller a page of the messenger host? Every command refuses anything else.
fn from_app(webview: &tauri::Webview) -> bool {
    match webview.url() {
        Ok(u) => guard::is_app_url(u.scheme(), u.host_str()),
        Err(_) => false,
    }
}

/// The page tells us how many chats are unread (it polls its own endpoint with its own login, so no credentials ever reach this process).
#[tauri::command]
async fn report_unread(app: tauri::AppHandle, webview: tauri::Webview, count: i64) -> Result<(), String> {
    if !from_app(&webview) {
        return Err("report_unread: not allowed from this page".to_string());
    }
    let new = UNREAD.record(count);
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(unread::tooltip(count.max(0))));
    }
    if new > 0 {
        diag::log(&format!("unread rose by {}", new));
        let _ = app.notification().builder().title("Evolve Messenger").body(unread::body(new)).show();
    }
    Ok(())
}

/// Links the page opens in a new window or to another site go to the system browser, never into this window.
#[tauri::command]
async fn open_external_url(app: tauri::AppHandle, webview: tauri::Webview, url: String) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;
    if !from_app(&webview) {
        return Err("open_external_url: not allowed from this page".to_string());
    }
    if !guard::is_openable_external(&url) {
        return Err("Only http/https/mailto links are supported".to_string());
    }
    app.shell().open(&url, None).map_err(|e| e.to_string())
}

fn show_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window(WINDOW) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Open the folder holding evolvemessenger.log in the system file manager (tray: "Open log folder").
fn open_log_folder() {
    let Some(dir) = diag::log_dir() else { return };
    #[cfg(target_os = "windows")]
    let opener = "explorer";
    #[cfg(target_os = "macos")]
    let opener = "open";
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let opener = "xdg-open";
    if let Err(e) = std::process::Command::new(opener).arg(&dir).spawn() {
        diag::log(&format!("could not open the log folder: {}", e));
    }
}

fn say(app: &tauri::AppHandle, text: &str) {
    let _ = app.notification().builder().title("Evolve Messenger").body(text).show();
}

async fn check_updates(app: tauri::AppHandle, manual: bool) {
    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            diag::log(&format!("update: updater unavailable: {}", e));
            return;
        }
    };
    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            diag::log(&format!("update: {} is available (current {})", version, env!("CARGO_PKG_VERSION")));
            if let Some(item) = app.try_state::<InstallItem>() {
                let _ = item.0.set_text(format!("Install update {} and restart", version));
                let _ = item.0.set_enabled(true);
            }
            *app.state::<PendingUpdate>().0.lock().unwrap() = Some(update);
            say(&app, &format!("Version {} is ready. Use the tray menu: Install update and restart.", version));
        }
        Ok(None) => {
            if manual {
                say(&app, "You are on the latest version.");
            }
        }
        Err(e) => {
            diag::log(&format!("update: check failed: {}", e));
            if manual {
                say(&app, update_error::user_text(update_error::Kind::Network));
            }
        }
    }
}

async fn install_update(app: tauri::AppHandle) {
    use update_error::{classify, encode, user_text, Stage};
    let update = app.state::<PendingUpdate>().0.lock().unwrap().take();
    let Some(update) = update else {
        say(&app, user_text(update_error::Kind::Consumed));
        return;
    };
    diag::log(&format!("update: installing {} (current {}, {} {})", update.version, env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH));
    let bytes = match update.download(|_, _| {}, || {}).await {
        Ok(b) => b,
        Err(e) => {
            let msg = e.to_string();
            let kind = classify(Stage::Download, &msg);
            diag::log(&format!("update: download failed: {}", encode(kind, &msg)));
            say(&app, user_text(kind));
            return;
        }
    };
    if let Err(e) = update.install(bytes) {
        let msg = e.to_string();
        let kind = classify(Stage::Install, &msg);
        diag::log(&format!("update: install failed: {}", encode(kind, &msg)));
        say(&app, user_text(kind));
        return;
    }
    diag::log("update: installed, restarting");
    app.restart();
}

/// Run inside the messenger page: tell the app how many chats are unread (the page's own login does the request).
const UNREAD_POLL_JS: &str = r#"
(async function() {
    if (!window.__TAURI_INTERNALS__ || location.hostname !== 'messager.evolvepreneuriq.app') return;
    try {
        var r = await fetch('/api/v1/desktop/check-notifications', {credentials: 'include'});
        if (r.ok) {
            var d = await r.json();
            window.__TAURI_INTERNALS__.invoke('report_unread', {count: parseInt(d.unread_chats, 10) || 0});
        }
    } catch (e) {}
})();
"#;

/// Run once per page load inside the messenger page: links to other sites and target=_blank links open in the system browser.
const EXTERNAL_LINKS_JS: &str = r#"
(function() {
    if (window.__evolve_messenger_links__) return;
    window.__evolve_messenger_links__ = true;
    document.addEventListener('click', function(e) {
        var el = e.target;
        while (el && el.tagName !== 'A') el = el.parentElement;
        if (!el || !el.href) return;
        try {
            var u = new URL(el.href);
            var internal = u.hostname === 'messager.evolvepreneuriq.app';
            if (!internal || el.target === '_blank') {
                if (u.protocol === 'https:' || u.protocol === 'http:' || u.protocol === 'mailto:') {
                    e.preventDefault();
                    window.__TAURI_INTERNALS__.invoke('open_external_url', {url: el.href});
                }
            }
        } catch (_) {}
    }, true);
})();
"#;

fn main() {
    tauri::Builder::default()
        // Must be the first plugin: a second launch focuses the running window instead of starting another tray, poll and updater.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_window(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![report_unread, open_external_url])
        .setup(|app| {
            let log_dir = app.path().app_log_dir().unwrap_or_else(|_| std::env::temp_dir().join("evolvemessenger-logs"));
            diag::init(log_dir);
            diag::log(&format!("start v{} {} {}", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH));
            {
                let h = app.handle().clone();
                diag::start_watchdog(move |job| {
                    let _ = h.run_on_main_thread(job);
                });
            }
            app.manage(PendingUpdate(Mutex::new(None)));

            // Started by "launch at login" (--hidden): stay in the tray until opened.
            let hidden = std::env::args().any(|a| a == "--hidden");
            let url = tauri::Url::parse(APP_URL).expect("APP_URL is a valid URL");
            let window = WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::External(url))
                .title("Evolve Messenger")
                .inner_size(420.0, 720.0)
                .min_inner_size(360.0, 480.0)
                .user_agent(&format!("EvolveMessenger/{} Tauri/2", env!("CARGO_PKG_VERSION")))
                .visible(!hidden)
                .on_page_load(|webview, payload| {
                    let u = payload.url();
                    if let PageLoadEvent::Finished = payload.event() {
                        diag::crumb(&format!("page-load finished {}{}", u.host_str().unwrap_or("?"), u.path()));
                        if guard::is_app_url(u.scheme(), u.host_str()) {
                            let _ = webview.eval(EXTERNAL_LINKS_JS);
                            let _ = webview.eval(UNREAD_POLL_JS);
                        }
                    }
                })
                .build()?;

            // The red X hides the window; the tray keeps the app (and its notifications) alive. Quit is in the tray menu.
            let w = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = w.hide();
                }
            });

            // Tray
            let launch_at_login = app.autolaunch().is_enabled().unwrap_or(false);
            let m_open = MenuItem::with_id(app, "open", "Open Evolve Messenger", true, None::<&str>)?;
            let m_login = CheckMenuItem::with_id(app, "launch_at_login", "Launch at login", true, launch_at_login, None::<&str>)?;
            let m_check = MenuItem::with_id(app, "check_updates", "Check for updates", true, None::<&str>)?;
            let m_install = MenuItem::with_id(app, "install_update", "No update to install", false, None::<&str>)?;
            let m_logs = MenuItem::with_id(app, "open_logs", "Open log folder (for support)", true, None::<&str>)?;
            let m_quit = MenuItem::with_id(app, "quit", "Quit Evolve Messenger", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&m_open, &m_login, &sep1, &m_check, &m_install, &m_logs, &sep2, &m_quit])?;
            app.manage(InstallItem(m_install.clone()));

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Evolve Messenger")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_window(app),
                    "launch_at_login" => {
                        let al = app.autolaunch();
                        let result = if al.is_enabled().unwrap_or(false) { al.disable() } else { al.enable() };
                        if let Err(e) = result {
                            diag::log(&format!("launch at login could not be changed: {}", e));
                        }
                    }
                    "check_updates" => {
                        let h = app.clone();
                        tauri::async_runtime::spawn(async move { check_updates(h, true).await });
                    }
                    "install_update" => {
                        let h = app.clone();
                        tauri::async_runtime::spawn(async move { install_update(h).await });
                    }
                    "open_logs" => open_log_folder(),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        show_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // Unread poll: every 60 s the messenger page is asked for its count (one timer; the page does the request with its own login).
            {
                let h = app.handle().clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(Duration::from_secs(60));
                    if let Some(w) = h.get_webview_window(WINDOW) {
                        let _ = w.eval(UNREAD_POLL_JS);
                    }
                });
            }
            // Update check: 30 s after start, then every 6 hours.
            {
                let h = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(30));
                    loop {
                        tauri::async_runtime::block_on(check_updates(h.clone(), false));
                        std::thread::sleep(Duration::from_secs(6 * 60 * 60));
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Evolve Messenger");
}
