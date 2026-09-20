use std::sync::mpsc;
use std::time::Duration;

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::component::progress::Progress;
use gpui_kit::component::{Disableable, StyledExt};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AppContext, ClickEvent, Context, Entity, IntoElement, ParentElement, Render, Styled, Window,
    div, px,
};

use crate::components::hash_options::HashOptions;
use crate::components::imsi_display::ImsiDisplay;
use crate::components::imsi_hash_file::ImsiHashFile;
use crate::imsi_search::{BruteForcer, SearchEvent, build_targets};

pub struct BruteForce {
    imsi: Entity<ImsiDisplay>,
    hash_options: Entity<HashOptions>,
    hash_file: Entity<ImsiHashFile>,
    output_state: Entity<TextareaState>,
    running: bool,
    found: Vec<String>,
    tried: u64,
    total: u64,
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
            tried: 0,
            total: 0,
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

        let targets = build_targets(
            self.hash_file.read(cx).hashes().iter().map(String::as_str),
            hex_mode,
        );

        self.found.clear();
        self.running = true;
        self.tried = 0;
        self.total = 0;

        if targets.is_empty() {
            self.running = false;
            self.set_output("No hash file loaded, or the file is empty.", window, cx);
            return;
        }

        self.set_output("Searching...", window, cx);

        let (tx, rx) = mpsc::channel::<SearchEvent>();

        cx.background_executor()
            .spawn(async move {
                let mut forcer = BruteForcer::new(&pattern, algorithm, encoding, targets);
                while let Some(event) = forcer.next_event() {
                    if tx.send(event).is_err() {
                        return;
                    }
                }
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
        batch: &[SearchEvent],
        done: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for event in batch {
            match event {
                SearchEvent::Match(m) => self.found.push(format!("{}  {}", m.imsi, m.hash)),
                SearchEvent::Progress { tried, total } => {
                    self.tried = *tried;
                    self.total = *total;
                }
            }
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

impl Render for BruteForce {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let label = if self.running {
            "Brute forcing..."
        } else {
            "Brute force"
        };

        let percent = if self.total > 0 {
            (self.tried as f64 / self.total as f64 * 100.0) as f32
        } else {
            0.0
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
            .when(self.running || self.total > 0, |this| {
                this.child(
                    div()
                        .v_flex()
                        .gap_1()
                        .child(Progress::new("brute-force-progress").value(percent))
                        .child(format!("{:.1}% ({}/{})", percent, self.tried, self.total)),
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
