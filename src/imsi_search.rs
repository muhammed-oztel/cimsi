use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::hashing::compute_hash;

/// The standard IMSI length: 3-digit MCC + 2/3-digit MNC + 9/10-digit MSIN.
pub const IMSI_TOTAL_LEN: usize = 15;

/// Where the known digits sit within the composed IMSI pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Prefix,
    Suffix,
}

/// Compose the full IMSI pattern: `prefix` (the operator's MCC+MNC) followed
/// or preceded by the known `digits`, padded with `*` for every unknown digit
/// so the result is always [`IMSI_TOTAL_LEN`] characters long.
pub fn compose_pattern(prefix: &str, digits: &str, position: Position) -> String {
    let remaining = IMSI_TOTAL_LEN.saturating_sub(prefix.len());
    let stars = "*".repeat(remaining.saturating_sub(digits.len()));

    match position {
        Position::Prefix => format!("{prefix}{digits}{stars}"),
        Position::Suffix => format!("{prefix}{stars}{digits}"),
    }
}

/// Split `0..total` into `threads` contiguous, roughly-equal sub-ranges (the
/// last range absorbs any remainder). `threads == 0` is treated as `1`.
pub fn split_ranges(total: u64, threads: usize) -> Vec<(u64, u64)> {
    let threads = threads.max(1) as u64;
    let chunk = total / threads;
    let mut ranges = Vec::with_capacity(threads as usize);
    let mut start = 0u64;

    for i in 0..threads {
        let end = if i == threads - 1 { total } else { start + chunk };
        ranges.push((start, end));
        start = end;
    }

    ranges
}

/// Build a hash lookup set from raw hash-file lines, normalizing case the way
/// [`compute_hash`]'s encodings compare: hex case-insensitively, base64 exactly.
pub fn build_targets<'a>(lines: impl IntoIterator<Item = &'a str>, hex_mode: bool) -> HashSet<String> {
    lines
        .into_iter()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| if hex_mode { line.to_lowercase() } else { line.to_string() })
        .collect()
}

#[derive(Debug, Clone)]
pub struct Match {
    pub imsi: String,
    pub hash: String,
}

/// Walks every digit combination for the `*` positions in an IMSI pattern,
/// hashing each candidate and yielding [`SearchEvent`]s: a [`Match`] whenever
/// a hash lands in the target set, and periodic progress updates.
///
/// Pull-based (via [`BruteForcer::next_event`]) so callers control pacing: a
/// GUI can pull a batch between cooperative yields, a CLI can just drain it
/// in a loop.
pub struct BruteForcer {
    candidate: Vec<u8>,
    wildcard_positions: Vec<usize>,
    algorithm: &'static str,
    encoding: &'static str,
    hex_mode: bool,
    targets: HashSet<String>,
    start: u64,
    end: u64,
    next_combo: u64,
    last_reported: u64,
    should_stop: Arc<AtomicBool>,
    finished: bool,
    thread_id: usize,
}

impl BruteForcer {
    /// Brute force only the combo-index sub-range `[start, end)` of `pattern`'s keyspace,
    /// letting multiple `BruteForcer`s split the same pattern's work across threads.
    /// `thread_id` is echoed back on every [`SearchEvent::Progress`] so an aggregator can
    /// tell instances apart; `should_stop` is checked every step so a run can be halted
    /// cooperatively.
    pub fn new_range(
        pattern: &str,
        algorithm: &'static str,
        encoding: &'static str,
        targets: HashSet<String>,
        start: u64,
        end: u64,
        thread_id: usize,
        should_stop: Arc<AtomicBool>,
    ) -> Self {
        let wildcard_positions: Vec<usize> = pattern
            .char_indices()
            .filter(|(_, c)| *c == '*')
            .map(|(i, _)| i)
            .collect();

        Self {
            candidate: pattern.as_bytes().to_vec(),
            wildcard_positions,
            algorithm,
            encoding,
            hex_mode: encoding == "Hex",
            targets,
            start,
            end,
            next_combo: start,
            last_reported: start,
            should_stop,
            finished: false,
            thread_id,
        }
    }

    /// Number of candidates already tried within this instance's range.
    pub fn tried(&self) -> u64 {
        self.next_combo - self.start
    }
}

/// One event yielded while draining a [`BruteForcer`] via
/// [`BruteForcer::next_event`]: either a hash match, or a progress update
/// reporting how many candidates have been tried so far.
///
/// Both variants carry `thread_id`/`tried` (relative to the emitting
/// `BruteForcer`'s own sub-range) so a caller can advance its saved
/// position on *every* event, not just `Progress` — otherwise a match found
/// between two progress ticks would still look "not yet reached" to a
/// checkpoint, and a resume would rewalk past it and report it again.
#[derive(Debug, Clone)]
pub enum SearchEvent {
    Match { thread_id: usize, tried: u64, found: Match },
    Progress { thread_id: usize, tried: u64 },
}

impl BruteForcer {
    /// Advance by exactly one candidate (or none, if stopped/exhausted) and
    /// report the result: a hash match, a periodic progress update every
    /// [`Self::PROGRESS_STEP`] candidates, or `None` once the range is
    /// finished — so long-running searches report percentage-complete
    /// without a caller having to poll `tried()` from another thread, and a
    /// cooperative stop is noticed within one candidate.
    pub fn next_event(&mut self) -> Option<SearchEvent> {
        const PROGRESS_STEP: u64 = 10_000;

        if self.finished {
            return None;
        }

        let thread_id = self.thread_id;

        loop {
            if self.next_combo >= self.end || self.should_stop.load(Ordering::Relaxed) {
                self.finished = true;
                return Some(SearchEvent::Progress { thread_id, tried: self.tried() });
            }

            let combo = self.next_combo;
            self.next_combo += 1;

            let mut remainder = combo;
            for &pos in &self.wildcard_positions {
                self.candidate[pos] = b'0' + (remainder % 10) as u8;
                remainder /= 10;
            }

            let imsi = String::from_utf8_lossy(&self.candidate).into_owned();
            let hash = compute_hash(&imsi, self.algorithm, self.encoding);
            let compare = if self.hex_mode { hash.to_lowercase() } else { hash.clone() };

            if self.targets.contains(&compare) {
                return Some(SearchEvent::Match { thread_id, tried: self.tried(), found: Match { imsi, hash } });
            }

            if self.next_combo >= self.end || self.next_combo - self.last_reported >= PROGRESS_STEP {
                self.last_reported = self.next_combo;
                self.finished = self.next_combo >= self.end;
                return Some(SearchEvent::Progress { thread_id, tried: self.tried() });
            }
        }
    }
}
