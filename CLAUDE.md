# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`cimsi` is a GSM IMSI hash brute-forcing tool built on `gpui-ce`/`gpui-kit` (the GPUI Component crate family), with a single Rust binary that runs either as a desktop GUI or headless CLI depending on argv.

Given a known operator (which fixes the IMSI's MCC+MNC prefix) and optionally some known digits, it composes a 15-digit IMSI pattern with `*` for the unknown digits, then brute-forces every digit combination, hashing each candidate with a chosen algorithm/encoding and checking it against a user-supplied list of target hashes.

## Commands

```bash
cargo build                          # build the binary
cargo check                          # fast type-check, use this while iterating
cargo run                            # launch the GUI (no subcommand = GUI)
cargo run -- list-operators          # CLI: print known operator codes + IMSI prefixes
cargo run -- brute-force --operator TCELL --digits 00000000 \
    --hash-file hashes.txt --algorithm md5 --encoding hex
```

There is no test suite yet. There's also no `cargo fmt`/`clippy` config beyond the toolchain defaults — `cargo check` is the fast feedback loop; format with `cargo fmt` before committing if touching a file broadly.

## Architecture

The crate is organized so the brute-force logic is independent of the GUI — that's what lets the CLI and GUI share code without duplicating it.

- **`src/main.rs`** — entry point. Parses argv with `clap` first: if a subcommand is present, dispatches to `cli::run()` and returns without ever touching `gpui` (so `cimsi list-operators` etc. can run headless/CI). No subcommand falls through to `run_gui()`, which does the usual `gpui_kit::application()` / `open_window` dance and mounts the `HelloWorld` root view (composes all the GUI components in `components/`).
- **`src/imsi/`** — pure domain data, no `gpui` dependency: `operator.rs` (`Operator`, the hardcoded `OPERATORS` list, `operators_for(country_code)`, `imsi_prefix_for(code)`) and `country.rs` (`Country`, `COUNTRIES`). Both GUI comboboxes and the CLI read from here.
- **`src/imsi_search.rs`** — the actual brute-force engine, pure and `gpui`-free:
  - `Position` (`Prefix`/`Suffix`) + `compose_pattern(prefix, digits, position)` build the 15-char IMSI pattern (`IMSI_TOTAL_LEN`), padding unknown digits with `*`.
  - `build_targets(lines, hex_mode)` normalizes hash-file lines into a `HashSet<String>` (hex compared case-insensitively, base64 exactly).
  - `BruteForcer` is a pull-based `Iterator<Item = Match>`: walks every digit combination for the `*` positions, hashes each candidate via `hashing::compute_hash`, and yields only the ones whose hash lands in the target set. Being an iterator (not a callback-driven loop) is deliberate — the CLI just does `for m in forcer { ... }`, while the GUI pulls from it on a background thread and streams matches to the UI over a channel.
- **`src/hashing.rs`** — `compute_hash(input, algorithm, encoding)`, pure. Algorithm is one of `"MD5"`/`"SHA-1"`/`"SHA-256"` (via `md5`, `sha1_smol`, `sha2`), encoding is `"Hex"` or `"Base64"` (via `base64`). These string constants are the contract between `hash_options.rs` (GUI dropdowns), `cli.rs` (clap enums), and `imsi_search`/`hashing` — if you add an algorithm/encoding, update all three.
- **`src/hash_file.rs`** — `read_hash_lines(path)` streams a hash-list file line-by-line via `BufReader` rather than `fs::read_to_string`-ing the whole thing, since these lists can run into the hundreds of thousands or millions of lines. In the GUI this is explicitly run on `cx.background_executor()` (not the foreground/UI executor) so loading a big file doesn't freeze the window.
- **`src/cli.rs`** — `clap`-derived `Cli`/`Command` (`brute-force`, `list-operators`). Thin: it only translates argv into calls against `imsi`, `imsi_search`, and `hash_file`, then prints matches (`imsi  hash`) to stdout and status to stderr, exiting non-zero if nothing was found.
- **`src/components/`** — GUI-only (`gpui-kit`/`gpui-component` views), each wrapping one piece of domain state as an `Entity` and wiring GPUI subscriptions between them rather than a shared app-state struct:
  - `country_combobox.rs` / `operator_combobox.rs` — cascading comboboxes; the operator combobox subscribes to the country combobox's `ComboboxState` via `cx.subscribe_in` and refilters its `SearchableVec` on change.
  - `imsi_display.rs` — owns the "known digits" input + Prefix/Suffix radio group, recomputes the composed pattern (via `imsi_search::compose_pattern`) on every operator/digit/position change, and exposes `pattern(cx)` for `brute_force.rs` to read.
  - `imsi_hash_file.rs` — native file picker (`cx.prompt_for_paths`) + background-thread file load, exposes `hashes()`.
  - `hash_options.rs` — algorithm/encoding comboboxes, exposes `algorithm(cx)`/`encoding(cx)` as the `&'static str` constants `hashing`/`imsi_search` expect.
  - `brute_force.rs` — the GUI orchestrator: on click, builds a `BruteForcer` from the other three components' current values, runs it on `cx.background_executor()`, and streams `Match`es back to a `Textarea` over an `mpsc` channel polled every 50ms via `cx.spawn_in`. This channel-plus-poll pattern (background executor for CPU work, foreground `cx.spawn_in`/`cx.update` for anything touching `Window`/`Entity` state) is the idiom to follow for any other long-running GUI task — GPUI's `AsyncApp`/entity updates aren't safe to call directly from a background thread, only plain data can cross the channel.

### Known gaps / in-progress state

- `src/imsi/operator.rs` ships a hand-written 6-entry `OPERATORS` list. Broader operator/country coverage would need extending that list (or sourcing it from elsewhere) in Rust.
- No parallelism in `BruteForcer` itself — only the file I/O and the fact that it runs off the GUI thread are optimized. For large wildcard counts (10+ unknown digits), the keyspace (`10^n`) dominates regardless of how fast the hash file loads.
