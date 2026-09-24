# cimsi

A GSM **IMSI hash brute-forcing** tool built with [`gpui-ce`](https://crates.io/crates/gpui-ce) / [`gpui-kit`](https://crates.io/crates/gpui-kit). A single Rust binary runs either as a desktop GUI or a headless CLI depending on its arguments.

An IMSI is the 15-digit identifier that identifies a subscriber on a mobile network. Its first digits are fixed by the operator (MCC + MNC prefix). Given a known operator and, optionally, some already-known digits, `cimsi` composes a 15-digit IMSI pattern with `*` for the unknown positions, then brute-forces every digit combination — hashing each candidate and checking the result against a list of target hashes.

> ⚠️ **Intended use.** This is a research and educational tool for recovering IMSIs from hash lists you are authorized to work with (e.g. auditing whether your own hashed identifiers are recoverable, CTFs, coursework). Only use it against data you own or have explicit permission to test.

## Build

```bash
cargo build --release      # optimized binary
cargo check                # fast type-check while iterating
```

## Usage

### GUI

```bash
cargo run
```

Launches the desktop app: pick a country and operator, enter known digits and their position, choose algorithm/encoding, load a hash file, and start. Matches stream into the results view as they are found.

### CLI

List known operators and their IMSI prefixes:

```bash
cargo run -- list-operators
```

Resume an interrupted run from its checkpoint:

```bash
cargo run -- brute-force --resume run.state
```

Matches print to stdout as `imsi  hash`; status and progress go to stderr. The process exits non-zero if no match is found.

### `brute-force` options

| Flag           | Description                                                                     | Default  |
| -------------- | ------------------------------------------------------------------------------- | -------- |
| `--operator`   | Operator code (see `list-operators`). Required unless `--resume`.               | —        |
| `--digits`     | Digits already known.                                                           | `""`     |
| `--position`   | Where known digits sit: `prefix` or `suffix`.                                   | `prefix` |
| `--algorithm`  | `md5`, `sha1`, or `sha256`.                                                     | `md5`    |
| `--encoding`   | `hex` or `base64`.                                                              | `hex`    |
| `--hash-file`  | Path to a text file of target hashes, one per line. Required unless `--resume`. | —        |
| `--threads`    | Threads to split the keyspace across.                                           | `1`      |
| `--state-file` | Periodically checkpoint progress here so the run can be resumed.                | off      |
| `--resume`     | Resume a previous run from a checkpoint written via `--state-file`.             | —        |
