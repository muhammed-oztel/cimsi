use std::path::PathBuf;

use gpui_kit::component::button::Button;
use gpui_kit::component::{Disableable, StyledExt};
use gpui_kit::{
    ClickEvent, Context, IntoElement, ParentElement, PathPromptOptions, Render, Styled, Window,
    div, px,
};

use crate::hash_file::read_hash_lines;

pub struct ImsiHashFile {
    path: Option<PathBuf>,
    hashes: Vec<String>,
    loading: bool,
}

impl ImsiHashFile {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            path: None,
            hashes: vec![],
            loading: false,
        }
    }

    pub fn hashes(&self) -> &[String] {
        &self.hashes
    }

    /// Display path of the currently loaded hash file, for embedding in a
    /// checkpoint so a resumed run can reload the same file.
    pub fn hashes_source_display(&self) -> String {
        self.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default()
    }

    fn pick_file(&mut self, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select IMSI hash file (.txt)".into()),
        });

        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(mut paths))) = paths.await else {
                return;
            };
            let Some(path) = paths.pop() else {
                return;
            };

            _ = this.update(cx, |this, cx| {
                this.loading = true;
                cx.notify();
            });

            // A hash-list file can run into the hundreds of thousands or
            // millions of lines, so parse it off the UI thread.
            let load_path = path.clone();
            let result = cx
                .background_executor()
                .spawn(async move { read_hash_lines(load_path) })
                .await;

            _ = this.update(cx, |this, cx| {
                this.loading = false;
                if let Ok(hashes) = result {
                    this.path = Some(path);
                    this.hashes = hashes;
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for ImsiHashFile {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = if self.loading {
            "Loading...".to_string()
        } else {
            match (&self.path, self.hashes.len()) {
                (Some(path), count) => format!(
                    "{} ({count} hashes)",
                    path.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                ),
                (None, _) => "No file selected".to_string(),
            }
        };

        div()
            .v_flex()
            .gap_2()
            .child(
                Button::new("select-imsi-hash-file")
                    .label("Select IMSI hash file...")
                    .disabled(self.loading)
                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                        this.pick_file(cx);
                    })),
            )
            .child(div().w(px(260.)).child(status))
    }
}
