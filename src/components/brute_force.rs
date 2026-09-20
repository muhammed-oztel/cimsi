use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::component::progress::Progress;
use gpui_kit::component::{Disableable, StyledExt};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AppContext, ClickEvent, Context, Entity, IntoElement, ParentElement, PathPromptOptions, Render,
    Styled, Window, div, px,
};

use crate::checkpoint::{Checkpoint, RangeProgress};
use crate::components::hash_options::HashOptions;
use crate::components::imsi_display::ImsiDisplay;
use crate::components::imsi_hash_file::ImsiHashFile;
use crate::hash_file::read_hash_lines;
use crate::imsi_search::{BruteForcer, Match, SearchEvent, build_targets, split_ranges};

/// One thread's assigned slice `[start, end)` (used for the percentage/total
/// calc), the absolute combo index this run's `BruteForcer` resumed from
/// (`resume`, used as the base for live progress — the forcer reports `tried`
/// relative to its own start, i.e. `resume`, not the slice's original
/// `start`), and the absolute combo index reached so far (`tried`).
struct ThreadProgress {
    start: u64,
    end: u64,
    resume: u64,
    tried: u64,
}

pub struct BruteForce {
    imsi: Entity<ImsiDisplay>,
    hash_options: Entity<HashOptions>,
    hash_file: Entity<ImsiHashFile>,
    output_state: Entity<TextareaState>,
    running: bool,
    found: Vec<Match>,
    threads: Vec<ThreadProgress>,
    should_stop: Arc<AtomicBool>,
    stop_requested: bool,
    run_started_at: Instant,
    /// Elapsed seconds at the moment the run stopped, frozen so rate/ETA
    /// don't keep drifting on redraws that happen after completion.
    frozen_elapsed: Option<f64>,
    already_tried_at_start: u64,
    pattern: String,
    algorithm: &'static str,
    encoding: &'static str,
    hash_file_display: String,
}

impl BruteForce {
    pub fn new(
        imsi: Entity<ImsiDisplay>,
        hash_options: Entity<HashOptions>,
        hash_file: Entity<ImsiHashFile>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let output_state =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("Matches will appear here..."));

        Self {
            imsi,
            hash_options,
            hash_file,
            output_state,
            running: false,
            found: vec![],
            threads: vec![],
            should_stop: Arc::new(AtomicBool::new(false)),
            stop_requested: false,
            run_started_at: Instant::now(),
            frozen_elapsed: None,
            already_tried_at_start: 0,
            pattern: String::new(),
            algorithm: "MD5",
            encoding: "Hex",
            hash_file_display: String::new(),
        }
    }

    fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.running {
            return;
        }

        let pattern = self.imsi.read(cx).pattern(cx).to_string();
        let algorithm = self.hash_options.read(cx).algorithm(cx);
        let encoding = self.hash_options.read(cx).encoding(cx);
        let hex_mode = encoding == "Hex";
        let thread_count = self.hash_options.read(cx).threads(cx);

        let targets = build_targets(
            self.hash_file.read(cx).hashes().iter().map(String::as_str),
            hex_mode,
        );

        if targets.is_empty() {
            self.set_output("No hash file loaded, or the file is empty.", window, cx);
            return;
        }

        let total = 10u64.saturating_pow(pattern.chars().filter(|c| *c == '*').count() as u32);
        let ranges: Vec<(u64, u64, u64)> = split_ranges(total, thread_count)
            .into_iter()
            .map(|(start, end)| (start, end, start))
            .collect();

        let hash_file_display = self.hash_file.read(cx).hashes_source_display();

        self.run_from_ranges(pattern, algorithm, encoding, targets, ranges, hash_file_display, Vec::new(), window, cx);
    }

    fn resume_from_checkpoint(
        &mut self,
        checkpoint: Checkpoint,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running {
            return;
        }

        let hex_mode = checkpoint.encoding == "Hex";
        let lines = match read_hash_lines(std::path::Path::new(&checkpoint.hash_file)) {
            Ok(lines) if !lines.is_empty() => lines,
            _ => {
                self.set_output(
                    &format!("Couldn't reload hash file: {}", checkpoint.hash_file),
                    window,
                    cx,
                );
                return;
            }
        };

        let targets = build_targets(lines.iter().map(String::as_str), hex_mode);
        let ranges = checkpoint.ranges.iter().map(|r| (r.start, r.end, r.tried)).collect();
        let hash_file_display = checkpoint.hash_file;

        self.run_from_ranges(
            checkpoint.pattern,
            checkpoint.algorithm,
            checkpoint.encoding,
            targets,
            ranges,
            hash_file_display,
            checkpoint.matches,
            window,
            cx,
        );
    }

    fn run_from_ranges(
        &mut self,
        pattern: String,
        algorithm: &'static str,
        encoding: &'static str,
        targets: std::collections::HashSet<String>,
        ranges: Vec<(u64, u64, u64)>,
        hash_file_display: String,
        initial_matches: Vec<Match>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.found = initial_matches;
        self.running = true;
        self.should_stop = Arc::new(AtomicBool::new(false));
        self.stop_requested = false;
        self.run_started_at = Instant::now();
        self.frozen_elapsed = None;
        self.already_tried_at_start = ranges.iter().map(|&(start, _, tried)| tried - start).sum();
        self.threads = ranges
            .iter()
            .map(|&(start, end, tried)| ThreadProgress { start, end, resume: tried, tried })
            .collect();
        self.pattern = pattern.clone();
        self.algorithm = algorithm;
        self.encoding = encoding;
        self.hash_file_display = hash_file_display;

        self.set_output("Searching...", window, cx);

        let (tx, rx) = mpsc::channel::<SearchEvent>();

        for (thread_id, &(_, end, resume)) in ranges.iter().enumerate() {
            let pattern = pattern.clone();
            let targets = targets.clone();
            let should_stop = Arc::clone(&self.should_stop);
            let tx = tx.clone();

            cx.background_executor()
                .spawn(async move {
                    let mut forcer =
                        BruteForcer::new_range(&pattern, algorithm, encoding, targets, resume, end, thread_id, should_stop);
                    while let Some(event) = forcer.next_event() {
                        if tx.send(event).is_err() {
                            return;
                        }
                    }
                })
                .detach();
        }
        drop(tx);

        cx.spawn_in(window, async move |this, cx| {
            loop {
                let mut batch = Vec::new();
                let disconnected = loop {
                    match rx.try_recv() {
                        Ok(item) => batch.push(item),
                        Err(mpsc::TryRecvError::Empty) => break false,
                        Err(mpsc::TryRecvError::Disconnected) => break true,
                    }
                };

                if !batch.is_empty() || disconnected {
                    let updated = cx.update(|window, app_cx| {
                        this.update(app_cx, |this, ctx| {
                            this.apply_batch(&batch, disconnected, window, ctx);
                        })
                    });
                    if updated.is_err() {
                        return;
                    }
                }

                if disconnected {
                    return;
                }

                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }
        })
        .detach();
    }

    fn stop(&mut self) {
        if !self.running {
            return;
        }
        self.stop_requested = true;
        self.should_stop.store(true, Ordering::Relaxed);
    }

    fn resume_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select checkpoint file".into()),
        });

        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(mut paths))) = paths.await else {
                return;
            };
            let Some(path) = paths.pop() else {
                return;
            };

            let checkpoint = cx
                .background_executor()
                .spawn(async move { Checkpoint::read(&path) })
                .await;

            let checkpoint = match checkpoint {
                Ok(c) => c,
                Err(err) => {
                    _ = cx.update(|window, app_cx| {
                        this.update(app_cx, |this, ctx| {
                            this.set_output(&format!("Failed to read checkpoint: {err}"), window, ctx);
                        })
                    });
                    return;
                }
            };

            _ = cx.update(|window, app_cx| {
                this.update(app_cx, |this, ctx| {
                    this.resume_from_checkpoint(checkpoint, window, ctx);
                })
            });
        })
        .detach();
    }

    fn save_checkpoint_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let checkpoint = Checkpoint {
            pattern: self.pattern.clone(),
            algorithm: self.algorithm,
            encoding: self.encoding,
            hash_file: self.hash_file_display.clone(),
            ranges: self
                .threads
                .iter()
                .map(|t| RangeProgress { start: t.start, end: t.end, tried: t.tried })
                .collect(),
            matches: self.found.clone(),
        };

        let directory = std::env::current_dir().unwrap_or_default();
        let receiver = cx.prompt_for_new_path(&directory, Some("checkpoint.txt"));

        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(path))) = receiver.await else {
                return;
            };

            let result = cx
                .background_executor()
                .spawn(async move { checkpoint.write(&path) })
                .await;

            if let Err(err) = result {
                _ = cx.update(|window, app_cx| {
                    this.update(app_cx, |this, ctx| {
                        this.set_output(&format!("Failed to save checkpoint: {err}"), window, ctx);
                    })
                });
            }
        })
        .detach();
    }

    fn apply_batch(
        &mut self,
        batch: &[SearchEvent],
        done: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for event in batch {
            match event {
                SearchEvent::Match { thread_id, tried, found } => {
                    if let Some(t) = self.threads.get_mut(*thread_id) {
                        t.tried = t.resume + tried;
                    }
                    // Guards against a stale/inconsistent checkpoint (saved
                    // position before an already-recorded match) causing a
                    // resumed search to rewalk past it and report it again.
                    let already_found = self.found.iter().any(|m| m.imsi == found.imsi && m.hash == found.hash);
                    if !already_found {
                        self.found.push(found.clone());
                    }
                }
                SearchEvent::Progress { thread_id, tried } => {
                    if let Some(t) = self.threads.get_mut(*thread_id) {
                        t.tried = t.resume + tried;
                    }
                }
            }
        }

        let was_stopped_manually = done && self.stop_requested;

        if done {
            self.running = false;
            self.stop_requested = false;
            self.frozen_elapsed = Some(self.run_started_at.elapsed().as_secs_f64().max(0.001));
        }

        let text = if self.found.is_empty() {
            if self.running {
                "Searching...".to_string()
            } else {
                "No matches found.".to_string()
            }
        } else {
            self.found.iter().map(|m| format!("{}  {}", m.imsi, m.hash)).collect::<Vec<_>>().join("\n")
        };

        self.set_output(&text, window, cx);

        if was_stopped_manually {
            self.save_checkpoint_dialog(window, cx);
        }
    }

    fn set_output(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.output_state
            .update(cx, |s, cx| s.set_value(text, window, cx));
        cx.notify();
    }

    fn total_all(&self) -> u64 {
        self.threads.iter().map(|t| t.end - t.start).sum()
    }

    fn tried_relative(&self) -> u64 {
        self.threads.iter().map(|t| t.tried - t.start).sum()
    }
}

impl Render for BruteForce {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let label = if self.running {
            "Brute forcing..."
        } else {
            "Brute force"
        };

        let total = self.total_all();
        let tried = self.tried_relative();
        let percent = if total > 0 { (tried as f64 / total as f64 * 100.0) as f32 } else { 0.0 };

        let elapsed = self
            .frozen_elapsed
            .unwrap_or_else(|| self.run_started_at.elapsed().as_secs_f64().max(0.001));
        let session_tried = tried.saturating_sub(self.already_tried_at_start);
        let rate = session_tried as f64 / elapsed;
        let remaining = total.saturating_sub(tried);
        let eta_text = if !self.running {
            "done".to_string()
        } else if rate > 0.0 {
            format_eta(remaining as f64 / rate)
        } else {
            "unknown".to_string()
        };

        div()
            .v_flex()
            .gap_2()
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .child(
                        Button::new("brute-force")
                            .label(label)
                            .danger()
                            .disabled(self.running)
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.start(window, cx);
                            })),
                    )
                    .when(self.running, |this| {
                        this.child(Button::new("brute-force-stop").label("Stop").on_click(
                            cx.listener(|this, _: &ClickEvent, _window, _cx| {
                                this.stop();
                            }),
                        ))
                    })
                    .when(!self.running, |this| {
                        this.child(Button::new("brute-force-resume").label("Resume from file...").on_click(
                            cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.resume_dialog(window, cx);
                            }),
                        ))
                    }),
            )
            .when(self.running || total > 0, |this| {
                this.child(
                    div()
                        .v_flex()
                        .gap_1()
                        .child(Progress::new("brute-force-progress").value(percent))
                        .child(format!(
                            "{:.1}% ({}/{})  |  {:.0} h/s  |  ETA {}",
                            percent, tried, total, rate, eta_text
                        )),
                )
            })
            .child(
                div()
                    .w(px(320.))
                    .h(px(160.))
                    .child(Textarea::new(&self.output_state).disabled(true).size_full()),
            )
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
