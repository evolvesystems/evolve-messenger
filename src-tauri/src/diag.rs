//! Diagnostics: a small rotating log file and a freeze watchdog. Std only (no new dependency), so it is unit-tested on its own.
//!
//! Why: a release build of Evolve Messenger has no console, so `println!` goes nowhere and a "the whole app stops responding" report had
//! nothing to look at. Now:
//!  - `log()` appends a timestamped line to `evolvemessenger.log` in the app's log folder (rotated at 1 MB to `evolvemessenger.log.1`, so at most ~2 MB)
//!    and keeps it in an in-memory ring of the last 40 events;
//!  - `crumb()` only adds to the ring (for frequent, low-value events such as polls and page loads);
//!  - a panic is written to the log with its location;
//!  - the watchdog asks the app's main (UI) thread to answer a ping every 5 s. If it does not answer within 10 s the log gets
//!    "MAIN THREAD NOT RESPONDING" with the last 40 events (what the app was doing right before it stuck), and later
//!    "main thread recovered after N s". If the log simply stops, the process was killed rather than frozen.
//!
//! Nothing personal is logged: no page query strings, no message text, no tokens (callers pass host + path only).

use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const FILE_NAME: &str = "evolvemessenger.log";
const MAX_BYTES: u64 = 1_000_000;
const RING: usize = 40;
/// Watchdog timing (production values; tests use short ones).
#[derive(Clone, Copy)]
pub struct Timing {
    pub ping: Duration,
    pub blocked_after: Duration,
    pub still_blocked_every: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Timing { ping: Duration::from_secs(5), blocked_after: Duration::from_secs(10), still_blocked_every: Duration::from_secs(60) }
    }
}

const SLOW_CALL: Duration = Duration::from_millis(1000);

struct Logger {
    dir: PathBuf,
    file: Option<File>,
    size: u64,
    ring: VecDeque<String>,
}

static LOGGER: OnceLock<Mutex<Logger>> = OnceLock::new();

/// UTC timestamp `2026-10-18T07:21:03Z` from epoch seconds (civil-from-days, no date crate).
pub fn timestamp(epoch_secs: u64) -> String {
    let days = (epoch_secs / 86_400) as i64;
    let rem = epoch_secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, rem / 3600, (rem % 3600) / 60, rem % 60)
}

fn now_stamp() -> String {
    timestamp(SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0))
}

/// Keep one line: no newlines, at most `limit` characters.
fn clean_to(msg: &str, limit: usize) -> String {
    msg.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).take(limit).collect()
}

fn clean(msg: &str) -> String {
    clean_to(msg, 300)
}

fn open_append(path: &PathBuf) -> Option<File> {
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// Start logging into `dir` (created if missing). Safe to call once; later calls are ignored. Never panics.
pub fn init(dir: PathBuf) {
    let _ = fs::create_dir_all(&dir);
    let path = dir.join(FILE_NAME);
    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let file = open_append(&path);
    let _ = LOGGER.set(Mutex::new(Logger { dir, file, size, ring: VecDeque::new() }));

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let place = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let what = info.payload().downcast_ref::<&str>().map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "panic".to_string());
        log(&format!("PANIC at {}: {}", place, what));
        previous(info);
    }));
}

/// The folder the log lives in (None before `init`).
pub fn log_dir() -> Option<PathBuf> {
    LOGGER.get().and_then(|m| m.lock().ok()).map(|l| l.dir.clone())
}

fn push_ring(l: &mut Logger, line: &str) {
    if l.ring.len() >= RING {
        l.ring.pop_front();
    }
    l.ring.push_back(line.to_string());
}

/// Add to the in-memory ring only (frequent events: polls, page loads, focus).
pub fn crumb(msg: &str) {
    if let Some(m) = LOGGER.get() {
        if let Ok(mut l) = m.lock() {
            let line = format!("{} {}", now_stamp(), clean(msg));
            push_ring(&mut l, &line);
        }
    }
}

/// Write a line to the log file (and the ring).
pub fn log(msg: &str) {
    log_limited(msg, 300);
}

/// Like `log` with a longer line allowed (the freeze report carries the last events).
fn log_limited(msg: &str, limit: usize) {
    let Some(m) = LOGGER.get() else { return };
    let Ok(mut l) = m.lock() else { return };
    let line = format!("{} {}", now_stamp(), clean_to(msg, limit));
    push_ring(&mut l, &line);
    if l.size + line.len() as u64 + 1 > MAX_BYTES {
        l.file = None;
        let current = l.dir.join(FILE_NAME);
        let _ = fs::rename(&current, l.dir.join(format!("{}.1", FILE_NAME)));
        l.file = open_append(&current);
        l.size = 0;
    }
    if let Some(f) = l.file.as_mut() {
        if writeln!(f, "{}", line).is_ok() {
            l.size += line.len() as u64 + 1;
        }
    }
}

/// The last events, oldest first (what the app was doing right before something happened).
pub fn recent() -> Vec<String> {
    LOGGER.get().and_then(|m| m.lock().ok()).map(|l| l.ring.iter().cloned().collect()).unwrap_or_default()
}

/// Run `f` and log it when it takes a second or more: the call that stalls the app shows up by name.
pub fn timed<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let out = f();
    let took = start.elapsed();
    if took >= SLOW_CALL {
        log(&format!("SLOW {} took {} ms", label, took.as_millis()));
    }
    out
}

/// What the watchdog decided on one ping (pure, so it is tested without threads).
#[derive(Debug, PartialEq)]
pub enum Verdict {
    /// Answered in time and nothing was wrong before.
    Fine,
    /// Answered, but only after being stuck for this long.
    Recovered(Duration),
    /// First time the ping went unanswered past the limit.
    Blocked,
    /// Still unanswered; log again only once per minute.
    StillBlocked(Duration),
    /// Still unanswered, nothing new to say yet.
    Quiet,
}

/// `answered`: did the ping come back; `blocked_for`: how long the main thread has been stuck so far (None = not stuck);
/// `since_last_report`: time since the last "still blocked" line.
pub fn judge(answered: bool, blocked_for: Option<Duration>, since_last_report: Duration, still_every: Duration) -> Verdict {
    match (answered, blocked_for) {
        (true, None) => Verdict::Fine,
        (true, Some(d)) => Verdict::Recovered(d),
        (false, None) => Verdict::Blocked,
        (false, Some(d)) if since_last_report >= still_every => Verdict::StillBlocked(d),
        (false, Some(_)) => Verdict::Quiet,
    }
}

/// Start the watchdog thread. `dispatch` must run the closure on the app's main thread (for Tauri:
/// `move |job| { let _ = handle.run_on_main_thread(job); }`).
pub fn start_watchdog<F>(dispatch: F)
where
    F: Fn(Box<dyn FnOnce() + Send + 'static>) + Send + 'static,
{
    start_watchdog_with(Timing::default(), dispatch);
}

/// Same with explicit timing (used by the tests).
pub fn start_watchdog_with<F>(t: Timing, dispatch: F)
where
    F: Fn(Box<dyn FnOnce() + Send + 'static>) + Send + 'static,
{
    std::thread::spawn(move || {
        let started = Instant::now();
        let mut last_alive_line = Instant::now();
        let mut blocked_since: Option<Instant> = None;
        let mut last_report = Instant::now();
        loop {
            std::thread::sleep(t.ping);
            let (tx, rx) = mpsc::channel::<()>();
            dispatch(Box::new(move || {
                let _ = tx.send(());
            }));
            // Disconnected = the job was dropped because the app is shutting down: nothing to watch any more.
            let mut answered = match rx.recv_timeout(t.blocked_after) {
                Ok(_) => true,
                Err(mpsc::RecvTimeoutError::Timeout) => false,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            };
            loop {
                let blocked_for = blocked_since.map(|s| s.elapsed());
                match judge(answered, blocked_for, last_report.elapsed(), t.still_blocked_every) {
                    Verdict::Fine | Verdict::Quiet => {}
                    Verdict::Recovered(d) => {
                        log(&format!("main thread recovered after {} s", d.as_secs().max(1)));
                        blocked_since = None;
                    }
                    Verdict::Blocked => {
                        blocked_since = Some(Instant::now() - t.blocked_after);
                        last_report = Instant::now();
                        // newest first, so a long list can never cut off the event that matters most
                        let events = recent().into_iter().rev().collect::<Vec<_>>().join(" | ");
                        log_limited(&format!("MAIN THREAD NOT RESPONDING for {} ms. Last events (newest first): {}", t.blocked_after.as_millis(), events), 8000);
                    }
                    Verdict::StillBlocked(d) => {
                        last_report = Instant::now();
                        log(&format!("main thread STILL not responding after {} s", d.as_secs()));
                    }
                }
                if answered || blocked_since.is_none() {
                    break;
                }
                // Stuck: keep waiting for the late answer, reporting once a minute.
                answered = match rx.recv_timeout(t.ping) {
                    Ok(_) => true,
                    Err(mpsc::RecvTimeoutError::Timeout) => false,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                };
            }
            if last_alive_line.elapsed() >= Duration::from_secs(300) {
                last_alive_line = Instant::now();
                log(&format!("alive, up {} min", started.elapsed().as_secs() / 60));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_matches_known_dates() {
        assert_eq!(timestamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(timestamp(951_782_400), "2000-02-29T00:00:00Z"); // leap day
        assert_eq!(timestamp(1_792_284_063), "2026-10-18T00:41:03Z");
        assert_eq!(timestamp(1_709_164_799), "2024-02-28T23:59:59Z");
    }

    #[test]
    fn clean_keeps_one_short_line() {
        assert_eq!(clean("a\nb\r\nc"), "a b  c");
        assert_eq!(clean(&"x".repeat(1000)).len(), 300);
    }

    #[test]
    fn judge_decides_each_case() {
        let s = Duration::from_secs;
        let every = s(60);
        assert_eq!(judge(true, None, s(0), every), Verdict::Fine);
        assert_eq!(judge(true, Some(s(42)), s(0), every), Verdict::Recovered(s(42)));
        assert_eq!(judge(false, None, s(0), every), Verdict::Blocked);
        assert_eq!(judge(false, Some(s(30)), s(10), every), Verdict::Quiet);
        assert_eq!(judge(false, Some(s(70)), s(60), every), Verdict::StillBlocked(s(70)));
    }

    /// The logger is process-wide: tests that read it run one at a time.
    fn serial() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// One logger per process: every test shares this folder.
    fn test_dir() -> PathBuf {
        static DIR: OnceLock<PathBuf> = OnceLock::new();
        DIR.get_or_init(|| {
            let d = std::env::temp_dir().join(format!("evolvemessenger-diag-test-{}", std::process::id()));
            let _ = fs::remove_dir_all(&d);
            init(d.clone());
            d
        })
        .clone()
    }

    #[test]
    fn log_writes_rotates_and_keeps_a_ring() {
        let _g = serial();
        let dir = test_dir();
        assert_eq!(log_dir(), Some(dir.clone()));
        log("first line");
        crumb("only in the ring");
        let text = fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        assert!(text.contains("first line"));
        assert!(!text.contains("only in the ring"));
        let ring = recent();
        assert!(ring.iter().any(|l| l.ends_with("first line")));
        assert!(ring.iter().any(|l| l.ends_with("only in the ring")));

        let line = "y".repeat(280);
        for _ in 0..(MAX_BYTES as usize / 280 + 50) {
            log(&line);
        }
        assert!(dir.join(format!("{}.1", FILE_NAME)).exists(), "rotated file exists");
        assert!(fs::metadata(dir.join(FILE_NAME)).unwrap().len() <= MAX_BYTES);
    }

    #[test]
    fn the_ring_keeps_only_the_last_events() {
        let _g = serial();
        test_dir();
        for i in 0..(RING + 10) {
            crumb(&format!("ring-test {}", i));
        }
        assert!(recent().len() <= RING);
    }

    #[test]
    fn timed_returns_the_value() {
        let _g = serial();
        test_dir();
        assert_eq!(timed("fast", || 7), 7);
    }

    /// The real thread: a "main thread" that stalls for a while, then answers.
    #[test]
    fn the_watchdog_reports_a_stalled_main_thread_with_the_last_events_then_the_recovery() {
        let _g = serial();
        let dir = test_dir();
        crumb("page-load Finished evolvepreneuriq.app/admin/lab");
        let stall_once = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let t = Timing { ping: Duration::from_millis(40), blocked_after: Duration::from_millis(150), still_blocked_every: Duration::from_millis(200) };
        let flag = stall_once.clone();
        start_watchdog_with(t, move |job| {
            let stall = flag.swap(false, std::sync::atomic::Ordering::SeqCst);
            std::thread::spawn(move || {
                if stall {
                    std::thread::sleep(Duration::from_millis(700)); // the stuck UI thread
                }
                job();
            });
        });
        std::thread::sleep(Duration::from_millis(1500));
        // (another test may have rotated the file meanwhile: read both)
        let text = fs::read_to_string(dir.join(FILE_NAME)).unwrap_or_default() + &fs::read_to_string(dir.join(format!("{}.1", FILE_NAME))).unwrap_or_default();
        assert!(text.contains("MAIN THREAD NOT RESPONDING"), "reported: {}", text.lines().rev().take(5).collect::<Vec<_>>().join(" / "));
        assert!(text.contains("page-load Finished"), "the report carries the last events");
        assert!(text.contains("main thread recovered after"), "and says when it came back");
    }

    #[test]
    fn a_dropped_ping_is_the_app_shutting_down_not_a_freeze() {
        let (tx, rx) = mpsc::channel::<()>();
        drop(tx);
        assert!(matches!(rx.recv_timeout(Duration::from_millis(50)), Err(mpsc::RecvTimeoutError::Disconnected)));
    }
}
