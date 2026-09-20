use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

/// Read a hash-list file (one hash per line) without ever holding the whole
/// file as a single string — important once these lists run into the
/// hundreds of thousands or millions of lines.
pub fn read_hash_lines(path: impl AsRef<Path>) -> io::Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            lines.push(trimmed.to_string());
        }
    }

    Ok(lines)
}
