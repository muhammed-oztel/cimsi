use std::collections::HashSet;
use std::sync::mpsc;
use std::time::Duration;

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::component::{Disableable, StyledExt};
use gpui_kit::{
    AppContext, ClickEvent, Context, Entity, IntoElement, ParentElement, Render, Styled, Window,
    div, px,
};

use crate::components::hash_options::HashOptions;
use crate::components::imsi_display::ImsiDisplay;
use crate::components::imsi_hash_file::ImsiHashFile;
use crate::hashing::compute_hash;

pub struct BruteForce {
    imsi: Entity<ImsiDisplay>,
    hash_options: Entity<HashOptions>,
    hash_file: Entity<ImsiHashFile>,
    output_state: Entity<TextareaState>,
    running: bool,
    found: Vec<String>,
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

        let targets: HashSet<String> = self
            .hash_file
            .read(cx)
            .hashes()
            .iter()
            .map(|h| {
                let h = h.trim().to_string();
                if hex_mode { h.to_lowercase() } else { h }
            })
            .filter(|h| !h.is_empty())
            .collect();

        self.found.clear();
        self.running = true;

        if targets.is_empty() {
            self.running = false;
            self.set_output("No hash file loaded, or the file is empty.", window, cx);
            return;
        }

        self.set_output("Searching...", window, cx);

        let wildcard_positions: Vec<usize> = pattern
            .char_indices()
            .filter(|(_, c)| *c == '*')
            .map(|(i, _)| i)
            .collect();

        let (tx, rx) = mpsc::channel::<(String, String)>();

        cx.background_executor()
            .spawn(async move {
                brute_force_worker(
                    pattern,
                    wildcard_positions,
                    algorithm,
                    encoding,
                    hex_mode,
                    targets,
                    tx,
                );
            })
            .detach();

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
                    let done = disconnected;
                    let updated = cx.update(|window, app_cx| {
                        this.update(app_cx, |this, ctx| {
                            this.apply_batch(&batch, done, window, ctx);
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

    fn apply_batch(
        &mut self,
        batch: &[(String, String)],
        done: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for (imsi, hash) in batch {
            self.found.push(format!("{imsi}  {hash}"));
        }

        if done {
            self.running = false;
        }

        let text = if self.found.is_empty() {
            if self.running {
                "Searching...".to_string()
            } else {
                "No matches found.".to_string()
            }
        } else {
            self.found.join("\n")
        };

        self.set_output(&text, window, cx);
    }

    fn set_output(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.output_state
            .update(cx, |s, cx| s.set_value(text, window, cx));
        cx.notify();
    }
}

fn brute_force_worker(
    pattern: String,
    wildcard_positions: Vec<usize>,
    algorithm: &'static str,
    encoding: &'static str,
    hex_mode: bool,
    targets: HashSet<String>,
    tx: mpsc::Sender<(String, String)>,
) {
    let wildcard_count = wildcard_positions.len() as u32;
    let total = 10u64.saturating_pow(wildcard_count);
    let mut candidate = pattern.into_bytes();

    for combo in 0..total {
        let mut remainder = combo;
        for &pos in &wildcard_positions {
            candidate[pos] = b'0' + (remainder % 10) as u8;
            remainder /= 10;
        }

        let imsi = String::from_utf8_lossy(&candidate).into_owned();
        let hash = compute_hash(&imsi, algorithm, encoding);
        let compare_hash = if hex_mode {
            hash.to_lowercase()
        } else {
            hash.clone()
        };

        if targets.contains(&compare_hash) && tx.send((imsi, hash)).is_err() {
            return;
        }
    }
}

impl Render for BruteForce {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let label = if self.running {
            "Brute forcing..."
        } else {
            "Brute force"
        };

        div()
            .v_flex()
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
            .child(
                div()
                    .w(px(320.))
                    .h(px(160.))
                    .child(Textarea::new(&self.output_state).disabled(true).size_full()),
            )
    }
}
