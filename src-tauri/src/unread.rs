//! The unread-chat count and when it should become a notification. std-only, tested without the Tauri toolchain.
//!
//! The first count only sets the baseline (opening the app never notifies for messages that were already waiting). After that a notification is due only when the
//! count RISES; reading messages lowers the baseline silently. Count only: no names or text (privacy, same rule as the EvolveApp email notification).

use std::sync::atomic::{AtomicI64, Ordering};

/// Largest count we trust from the page (a hostile or broken page cannot make us announce a billion messages).
const MAX_COUNT: i64 = 100_000;

pub struct Tracker {
    last: AtomicI64,
}

impl Tracker {
    pub const fn new() -> Self {
        Tracker { last: AtomicI64::new(-1) }
    }

    /// Record the latest count; returns how many NEW messages to announce (0 = nothing).
    pub fn record(&self, now: i64) -> i64 {
        let now = now.clamp(0, MAX_COUNT);
        let prev = self.last.swap(now, Ordering::SeqCst);
        if prev < 0 || now <= prev { 0 } else { now - prev }
    }
}

pub fn body(new: i64) -> String {
    if new == 1 { "You have 1 new message".to_string() } else { format!("You have {} new messages", new) }
}

pub fn tooltip(total: i64) -> String {
    if total > 0 { format!("Evolve Messenger - {} unread", total) } else { "Evolve Messenger".to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_count_is_only_a_baseline() {
        let t = Tracker::new();
        assert_eq!(t.record(7), 0);
    }

    #[test]
    fn a_rise_announces_exactly_the_difference() {
        let t = Tracker::new();
        t.record(2);
        assert_eq!(t.record(5), 3);
        assert_eq!(t.record(6), 1);
    }

    #[test]
    fn the_same_count_again_announces_nothing() {
        let t = Tracker::new();
        t.record(4);
        assert_eq!(t.record(4), 0);
    }

    #[test]
    fn reading_messages_lowers_the_baseline_so_the_next_new_one_is_announced() {
        let t = Tracker::new();
        t.record(5);
        assert_eq!(t.record(1), 0, "a fall is silent");
        assert_eq!(t.record(2), 1, "one new message after reading");
    }

    #[test]
    fn nonsense_counts_are_clamped() {
        let t = Tracker::new();
        t.record(-5);
        assert_eq!(t.record(i64::MAX), 100_000);
        let t2 = Tracker::new();
        assert_eq!(t2.record(-9), 0);
        assert_eq!(t2.record(0), 0);
    }

    #[test]
    fn the_texts_are_count_only() {
        assert_eq!(body(1), "You have 1 new message");
        assert_eq!(body(3), "You have 3 new messages");
        assert_eq!(tooltip(0), "Evolve Messenger");
        assert_eq!(tooltip(4), "Evolve Messenger - 4 unread");
    }
}
