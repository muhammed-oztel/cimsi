use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::checkpoint::{Checkpoint, RangeProgress};
use crate::hash_file::read_hash_lines;
use crate::imsi::{OPERATORS, imsi_prefix_for};
use crate::imsi_search::{BruteForcer, Position, SearchEvent, build_targets, compose_pattern, split_ranges};

#[derive(Parser)]
#[command(name = "cimsi", about = "IMSI hash brute-forcing tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Brute force IMSI candidates against a hash list.
    BruteForce(BruteForceArgs),
    /// List known GSM operators and their IMSI prefixes.
    ListOperators,
}

#[derive(Args)]
pub struct BruteForceArgs {
    /// Operator code, e.g. TCELL, VF_TR, ATT (see `list-operators`). Ignored with --resume.
    #[arg(long, required_unless_present = "resume", conflicts_with = "resume")]
    operator: Option<String>,

    /// Digits already known. Ignored with --resume.
    #[arg(long, default_value = "", conflicts_with = "resume")]
    digits: String,

    /// Where the known digits sit within the IMSI. Ignored with --resume.
    #[arg(long, value_enum, default_value_t = PositionArg::Prefix, conflicts_with = "resume")]
    position: PositionArg,

    /// Hash algorithm the target list was generated with. Ignored with --resume.
    #[arg(long, value_enum, default_value_t = AlgorithmArg::Md5, conflicts_with = "resume")]
    algorithm: AlgorithmArg,

    /// Output encoding of the target list's hashes. Ignored with --resume.
    #[arg(long, value_enum, default_value_t = EncodingArg::Hex, conflicts_with = "resume")]
    encoding: EncodingArg,

    /// Path to a text file of target hashes, one per line. Ignored with --resume.
    #[arg(long, required_unless_present = "resume", conflicts_with = "resume")]
    hash_file: Option<PathBuf>,

    /// Number of threads to split the keyspace across.
    #[arg(long, default_value_t = 1)]
    threads: usize,

    /// Periodically checkpoint progress to this file so the run can be resumed later.
    #[arg(long)]
    state_file: Option<PathBuf>,

    /// Resume a previous run from a checkpoint file written via --state-file.
    #[arg(long)]
    resume: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
enum PositionArg {
    Prefix,
    Suffix,
}

#[derive(Clone, Copy, ValueEnum)]
enum AlgorithmArg {
    Md5,
    Sha1,
    Sha256,
}

#[derive(Clone, Copy, ValueEnum)]
enum EncodingArg {
    Hex,
    Base64,
}

impl PositionArg {
    fn to_domain(self) -> Position {
        match self {
            Self::Prefix => Position::Prefix,
            Self::Suffix => Position::Suffix,
        }
    }
}

impl AlgorithmArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::Md5 => "MD5",
            Self::Sha1 => "SHA-1",
            Self::Sha256 => "SHA-256",
        }
    }
}

impl EncodingArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::Hex => "Hex",
            Self::Base64 => "Base64",
        }
    }
}

pub fn run(command: Command) -> ExitCode {
    match command {
        Command::ListOperators => {
            for op in OPERATORS {
                println!("{:<8} {:<20} {}", op.code, op.name, op.imsi_prefix);
            }
            ExitCode::SUCCESS
        }
        Command::BruteForce(args) => run_brute_force(args),
    }
}

fn run_brute_force(args: BruteForceArgs) -> ExitCode {
    if let Some(resume_path) = args.resume.clone() {
        return run_resumed(&resume_path);
    }

    // `required_unless_present = "resume"` guarantees these are `Some` here.
    let operator = args.operator.as_deref().unwrap();
    let hash_file = args.hash_file.as_ref().unwrap();

    let Some(prefix) = imsi_prefix_for(operator) else {
        eprintln!("Unknown operator code '{operator}'. Run `cimsi list-operators` to see valid codes.");
        return ExitCode::FAILURE;
    };

    if !args.digits.chars().all(|c| c.is_ascii_digit()) {
        eprintln!("--digits must contain only digits.");
        return ExitCode::FAILURE;
    }

    let pattern = compose_pattern(prefix, &args.digits, args.position.to_domain());

    let lines = match read_hash_lines(hash_file) {
        Ok(lines) => lines,
        Err(err) => {
            eprintln!("Failed to read {}: {err}", hash_file.display());
            return ExitCode::FAILURE;
        }
    };

    if lines.is_empty() {
        eprintln!("Hash file is empty.");
        return ExitCode::FAILURE;
    }

    let hex_mode = matches!(args.encoding, EncodingArg::Hex);
    let targets = build_targets(lines.iter().map(String::as_str), hex_mode);

    let total = 10u64.saturating_pow(pattern.chars().filter(|c| *c == '*').count() as u32);
    let ranges: Vec<(u64, u64, u64)> = split_ranges(total, args.threads)
        .into_iter()
        .map(|(start, end)| (start, end, start))
        .collect();

    eprintln!(
        "Pattern: {pattern}  |  {total} candidates  |  {} thread(s)  |  {} target hashes",
        ranges.len(),
        lines.len(),
    );

    run_ranges(
        &pattern,
        args.algorithm.as_str(),
        args.encoding.as_str(),
        &hash_file.display().to_string(),
        targets,
        ranges,
        Vec::new(),
        args.state_file,
    )
}

fn run_resumed(resume_path: &PathBuf) -> ExitCode {
    let checkpoint = match Checkpoint::read(resume_path) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("Failed to read checkpoint {}: {err}", resume_path.display());
            return ExitCode::FAILURE;
        }
    };

    let lines = match read_hash_lines(std::path::Path::new(&checkpoint.hash_file)) {
        Ok(lines) => lines,
        Err(err) => {
            eprintln!("Failed to read {}: {err}", checkpoint.hash_file);
            return ExitCode::FAILURE;
        }
    };

    if lines.is_empty() {
        eprintln!("Hash file is empty.");
        return ExitCode::FAILURE;
    }

    let hex_mode = checkpoint.encoding == "Hex";
    let targets = build_targets(lines.iter().map(String::as_str), hex_mode);

    let ranges: Vec<(u64, u64, u64)> = checkpoint
        .ranges
        .iter()
        .map(|r| (r.start, r.end, r.tried))
        .collect();

    eprintln!(
        "Resuming pattern: {}  |  {} thread(s)  |  {} target hashes",
        checkpoint.pattern,
        ranges.len(),
        lines.len(),
    );

    run_ranges(
        &checkpoint.pattern,
        checkpoint.algorithm,
        checkpoint.encoding,
        &checkpoint.hash_file,
        targets,
        ranges,
        checkpoint.matches,
        Some(resume_path.clone()),
    )
}

/// Drive one brute-force run across `ranges.len()` threads, each walking the
/// `(orig_start, end, resume_point)` slice assigned to it — starting at
/// `resume_point` rather than `orig_start` so a resumed run skips work
/// already done in a previous process. Prints matches as found, periodic
/// progress with rate/ETA, and (if `state_file` is set) a checkpoint every
/// ~1s plus a final one at the end.
fn run_ranges(
    pattern: &str,
    algorithm: &'static str,
    encoding: &'static str,
    hash_file_display: &str,
    targets: std::collections::HashSet<String>,
    ranges: Vec<(u64, u64, u64)>,
    initial_matches: Vec<crate::imsi_search::Match>,
    state_file: Option<PathBuf>,
) -> ExitCode {
    let should_stop = Arc::new(AtomicBool::new(false));
    let tried_counters: Vec<AtomicU64> = ranges.iter().map(|&(_, _, resume)| AtomicU64::new(resume)).collect();

    let total_all: u64 = ranges.iter().map(|&(start, end, _)| end - start).sum();
    let already_tried: u64 = ranges.iter().map(|&(start, _, resume)| resume - start).sum();

    for m in &initial_matches {
        println!("{}  {}", m.imsi, m.hash);
    }

    // Match events carry (thread_id, tried, Match) so the aggregator below can
    // advance `tried_counters` for a match in the same step it records the
    // match — updating the counter from the worker thread instead would risk
    // a checkpoint landing between "position advanced" and "match recorded",
    // silently dropping that match on a later resume.
    let (tx, rx) = mpsc::channel::<(usize, u64, crate::imsi_search::Match)>();
    let start_time = Instant::now();
    let mut found_matches = initial_matches;

    std::thread::scope(|scope| {
        for (thread_id, &(_, end, resume)) in ranges.iter().enumerate() {
            let targets = targets.clone();
            let should_stop = Arc::clone(&should_stop);
            let tx = tx.clone();
            let tried_counter = &tried_counters[thread_id];

            scope.spawn(move || {
                let mut forcer =
                    BruteForcer::new_range(pattern, algorithm, encoding, targets, resume, end, thread_id, should_stop);

                while let Some(event) = forcer.next_event() {
                    match event {
                        SearchEvent::Match { thread_id, tried, found } => {
                            if tx.send((thread_id, tried, found)).is_err() {
                                break;
                            }
                        }
                        SearchEvent::Progress { tried, .. } => {
                            tried_counter.store(resume + tried, Ordering::Relaxed);
                        }
                    }
                }
            });
        }
        drop(tx);

        let mut last_checkpoint = Instant::now();

        // Guards against a stale/inconsistent checkpoint (saved position
        // before an already-recorded match) causing a resumed search to
        // rewalk past it and report it again.
        let record_match = |found_matches: &mut Vec<crate::imsi_search::Match>, thread_id: usize, tried: u64, m: crate::imsi_search::Match| {
            tried_counters[thread_id].store(ranges[thread_id].2 + tried, Ordering::Relaxed);
            let already_found = found_matches.iter().any(|f| f.imsi == m.imsi && f.hash == m.hash);
            if !already_found {
                println!("{}  {}", m.imsi, m.hash);
                found_matches.push(m);
            }
        };

        loop {
            match rx.recv_timeout(Duration::from_millis(250)) {
                Ok((thread_id, tried, m)) => {
                    record_match(&mut found_matches, thread_id, tried, m);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    while let Ok((thread_id, tried, m)) = rx.try_recv() {
                        record_match(&mut found_matches, thread_id, tried, m);
                    }
                    break;
                }
            }

            let tried_relative: u64 = ranges
                .iter()
                .zip(&tried_counters)
                .map(|(&(start, _, _), counter)| counter.load(Ordering::Relaxed) - start)
                .sum();

            let elapsed = start_time.elapsed().as_secs_f64().max(0.001);
            let session_tried = tried_relative.saturating_sub(already_tried);
            let rate = session_tried as f64 / elapsed;
            let remaining = total_all.saturating_sub(tried_relative);
            let eta = if rate > 0.0 { remaining as f64 / rate } else { f64::INFINITY };

            eprint!(
                "\r{:.1}%  ({tried_relative}/{total_all})  {rate:.0} h/s  ETA {}          ",
                tried_relative as f64 / total_all.max(1) as f64 * 100.0,
                format_eta(eta),
            );

            if let Some(path) = &state_file {
                if last_checkpoint.elapsed() >= Duration::from_secs(1) {
                    write_checkpoint(path, pattern, algorithm, encoding, hash_file_display, &ranges, &tried_counters, &found_matches);
                    last_checkpoint = Instant::now();
                }
            }
        }
    });

    eprintln!();

    if let Some(path) = &state_file {
        write_checkpoint(path, pattern, algorithm, encoding, hash_file_display, &ranges, &tried_counters, &found_matches);
    }

    eprintln!("Done. {} match(es) found.", found_matches.len());

    if !found_matches.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn write_checkpoint(
    path: &PathBuf,
    pattern: &str,
    algorithm: &'static str,
    encoding: &'static str,
    hash_file_display: &str,
    ranges: &[(u64, u64, u64)],
    tried_counters: &[AtomicU64],
    matches: &[crate::imsi_search::Match],
) {
    let checkpoint = Checkpoint {
        pattern: pattern.to_string(),
        algorithm,
        encoding,
        hash_file: hash_file_display.to_string(),
        ranges: ranges
            .iter()
            .zip(tried_counters)
            .map(|(&(start, end, _), counter)| RangeProgress {
                start,
                end,
                tried: counter.load(Ordering::Relaxed),
            })
            .collect(),
        matches: matches.to_vec(),
    };

    if let Err(err) = checkpoint.write(path) {
        eprintln!("Failed to write checkpoint {}: {err}", path.display());
    }
}

fn format_eta(seconds: f64) -> String {
    if !seconds.is_finite() {
        return "unknown".to_string();
    }

    let seconds = seconds as u64;
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    format!("{h:02}:{m:02}:{s:02}")
}
