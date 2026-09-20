use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::hash_file::read_hash_lines;
use crate::imsi::{OPERATORS, imsi_prefix_for};
use crate::imsi_search::{BruteForcer, Position, build_targets, compose_pattern};

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
    /// Operator code, e.g. TCELL, VF_TR, ATT (see `list-operators`).
    #[arg(long)]
    operator: String,

    /// Digits already known.
    #[arg(long, default_value = "")]
    digits: String,

    /// Where the known digits sit within the IMSI.
    #[arg(long, value_enum, default_value_t = PositionArg::Prefix)]
    position: PositionArg,

    /// Hash algorithm the target list was generated with.
    #[arg(long, value_enum, default_value_t = AlgorithmArg::Md5)]
    algorithm: AlgorithmArg,

    /// Output encoding of the target list's hashes.
    #[arg(long, value_enum, default_value_t = EncodingArg::Hex)]
    encoding: EncodingArg,

    /// Path to a text file of target hashes, one per line.
    #[arg(long)]
    hash_file: PathBuf,
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
    let Some(prefix) = imsi_prefix_for(&args.operator) else {
        eprintln!(
            "Unknown operator code '{}'. Run `cimsi list-operators` to see valid codes.",
            args.operator
        );
        return ExitCode::FAILURE;
    };

    if !args.digits.chars().all(|c| c.is_ascii_digit()) {
        eprintln!("--digits must contain only digits.");
        return ExitCode::FAILURE;
    }

    let pattern = compose_pattern(prefix, &args.digits, args.position.to_domain());

    let lines = match read_hash_lines(&args.hash_file) {
        Ok(lines) => lines,
        Err(err) => {
            eprintln!("Failed to read {}: {err}", args.hash_file.display());
            return ExitCode::FAILURE;
        }
    };

    if lines.is_empty() {
        eprintln!("Hash file is empty.");
        return ExitCode::FAILURE;
    }

    let hex_mode = matches!(args.encoding, EncodingArg::Hex);
    let targets = build_targets(lines.iter().map(String::as_str), hex_mode);

    let forcer = BruteForcer::new(&pattern, args.algorithm.as_str(), args.encoding.as_str(), targets);

    eprintln!(
        "Pattern: {pattern}  |  {} candidates  |  {} target hashes",
        forcer.total_combinations(),
        lines.len(),
    );

    let mut found = 0u64;
    for m in forcer {
        println!("{}  {}", m.imsi, m.hash);
        found += 1;
    }

    eprintln!("Done. {found} match(es) found.");

    if found > 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
