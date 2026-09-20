use std::fs;
use std::io;
use std::path::Path;

use crate::imsi_search::Match;

/// One thread's slice of the keyspace and how far it had gotten.
#[derive(Debug, Clone, Copy)]
pub struct RangeProgress {
    pub start: u64,
    pub end: u64,
    pub tried: u64,
}

/// A brute-force run's resumable state: everything needed to rebuild the
/// same `BruteForcer`s, continue where each left off, and show matches found
/// before the stop.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub pattern: String,
    pub algorithm: &'static str,
    pub encoding: &'static str,
    pub hash_file: String,
    pub ranges: Vec<RangeProgress>,
    pub matches: Vec<Match>,
}

impl Checkpoint {
    /// Serialize to the `key=value` text format and write it atomically
    /// (write to `<path>.tmp`, then rename over `path`).
    pub fn write(&self, path: &Path) -> io::Result<()> {
        let mut out = String::new();
        out.push_str(&format!("pattern={}\n", self.pattern));
        out.push_str(&format!("algorithm={}\n", self.algorithm));
        out.push_str(&format!("encoding={}\n", self.encoding));
        out.push_str(&format!("hash_file={}\n", self.hash_file));
        out.push_str(&format!("threads={}\n", self.ranges.len()));
        for r in &self.ranges {
            out.push_str(&format!("range={},{},{}\n", r.start, r.end, r.tried));
        }
        for m in &self.matches {
            out.push_str(&format!("match={},{}\n", m.imsi, m.hash));
        }

        let tmp_path = path.with_extension("tmp");
        fs::write(&tmp_path, out)?;
        fs::rename(&tmp_path, path)
    }

    /// Parse the `key=value` text format written by [`Checkpoint::write`].
    pub fn read(path: &Path) -> io::Result<Checkpoint> {
        let text = fs::read_to_string(path)?;

        let mut pattern = None;
        let mut algorithm = None;
        let mut encoding = None;
        let mut hash_file = None;
        let mut threads = None;
        let mut ranges = Vec::new();
        let mut matches = Vec::new();

        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            match key {
                "pattern" => pattern = Some(value.to_string()),
                "algorithm" => algorithm = Some(parse_algorithm(value)?),
                "encoding" => encoding = Some(parse_encoding(value)?),
                "hash_file" => hash_file = Some(value.to_string()),
                "threads" => {
                    threads = Some(value.parse::<usize>().map_err(|_| invalid("bad threads value"))?)
                }
                "range" => ranges.push(parse_range(value)?),
                "match" => matches.push(parse_match(value)?),
                _ => {}
            }
        }

        let pattern = pattern.ok_or_else(|| invalid("missing pattern"))?;
        let algorithm = algorithm.ok_or_else(|| invalid("missing algorithm"))?;
        let encoding = encoding.ok_or_else(|| invalid("missing encoding"))?;
        let hash_file = hash_file.ok_or_else(|| invalid("missing hash_file"))?;
        let threads = threads.ok_or_else(|| invalid("missing threads"))?;

        if ranges.len() != threads {
            return Err(invalid("threads count disagrees with number of range lines"));
        }

        Ok(Checkpoint {
            pattern,
            algorithm,
            encoding,
            hash_file,
            ranges,
            matches,
        })
    }
}

fn parse_range(value: &str) -> io::Result<RangeProgress> {
    let parts: Vec<&str> = value.split(',').collect();
    let [start, end, tried] = parts.as_slice() else {
        return Err(invalid("malformed range line"));
    };

    let start = start.parse().map_err(|_| invalid("malformed range start"))?;
    let end = end.parse().map_err(|_| invalid("malformed range end"))?;
    let tried = tried.parse().map_err(|_| invalid("malformed range tried"))?;

    Ok(RangeProgress { start, end, tried })
}

fn parse_match(value: &str) -> io::Result<Match> {
    let (imsi, hash) = value.split_once(',').ok_or_else(|| invalid("malformed match line"))?;
    Ok(Match { imsi: imsi.to_string(), hash: hash.to_string() })
}

fn parse_algorithm(value: &str) -> io::Result<&'static str> {
    match value {
        "MD5" => Ok("MD5"),
        "SHA-1" => Ok("SHA-1"),
        "SHA-256" => Ok("SHA-256"),
        _ => Err(invalid("unknown algorithm")),
    }
}

fn parse_encoding(value: &str) -> io::Result<&'static str> {
    match value {
        "Hex" => Ok("Hex"),
        "Base64" => Ok("Base64"),
        _ => Err(invalid("unknown encoding")),
    }
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("malformed checkpoint file: {msg}"))
}
