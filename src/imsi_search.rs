use std::collections::HashSet;

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
/// hashing each candidate and yielding it as a [`Match`] whenever its hash
/// lands in the target set.
///
/// Pull-based so callers control pacing: a GUI can pull a batch between
/// cooperative yields, a CLI can just drain it with a `for` loop.
pub struct BruteForcer {
    candidate: Vec<u8>,
    wildcard_positions: Vec<usize>,
    algorithm: &'static str,
    encoding: &'static str,
    hex_mode: bool,
    targets: HashSet<String>,
    next_combo: u64,
    total: u64,
    last_reported: u64,
}

impl BruteForcer {
    pub fn new(pattern: &str, algorithm: &'static str, encoding: &'static str, targets: HashSet<String>) -> Self {
        let wildcard_positions: Vec<usize> = pattern
            .char_indices()
            .filter(|(_, c)| *c == '*')
            .map(|(i, _)| i)
            .collect();

        let total = 10u64.saturating_pow(wildcard_positions.len() as u32);

        Self {
            candidate: pattern.as_bytes().to_vec(),
            wildcard_positions,
            algorithm,
            encoding,
            hex_mode: encoding == "Hex",
            targets,
            next_combo: 0,
            total,
            last_reported: 0,
        }
    }

    /// Total number of candidates this search will walk.
    pub fn total_combinations(&self) -> u64 {
        self.total
    }

    /// Number of candidates already tried.
    pub fn tried(&self) -> u64 {
        self.next_combo
    }
}

/// One event yielded while draining a [`BruteForcer`] via
/// [`BruteForcer::next_event`]: either a hash match, or a progress update
/// reporting how many candidates have been tried so far.
#[derive(Debug, Clone)]
pub enum SearchEvent {
    Match(Match),
    Progress { tried: u64, total: u64 },
}

impl BruteForcer {
    /// Like [`Iterator::next`], but also periodically yields a [`SearchEvent::Progress`]
    /// so long-running searches can report percentage-complete without a caller
    /// having to poll `tried()`/`total_combinations()` from another thread.
    pub fn next_event(&mut self) -> Option<SearchEvent> {
        const PROGRESS_STEP: u64 = 10_000;

        if self.next_combo >= self.total {
            if self.last_reported < self.total {
                self.last_reported = self.total;
                return Some(SearchEvent::Progress {
                    tried: self.total,
                    total: self.total,
                });
            }
            return None;
        }

        if self.next_combo - self.last_reported >= PROGRESS_STEP {
            self.last_reported = self.next_combo;
            return Some(SearchEvent::Progress {
                tried: self.next_combo,
                total: self.total,
            });
        }

        match self.next() {
            Some(m) => Some(SearchEvent::Match(m)),
            None => {
                self.last_reported = self.total;
                Some(SearchEvent::Progress {
                    tried: self.total,
                    total: self.total,
                })
            }
        }
    }
}

impl Iterator for BruteForcer {
    type Item = Match;

    fn next(&mut self) -> Option<Match> {
        while self.next_combo < self.total {
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
                return Some(Match { imsi, hash });
            }
        }

        None
    }
}
