use std::fs;
use std::path::PathBuf;

use gpui_kit::component::button::Button;
use gpui_kit::component::StyledExt;
use gpui_kit::{
    ClickEvent, Context, IntoElement, ParentElement, PathPromptOptions, Render, SharedString,
    Styled, Window, div, px,
};

pub struct ImsiHashFile {
    path: Option<PathBuf>,
    hashes: Vec<SharedString>,
}

impl ImsiHashFile {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            path: None,
            hashes: vec![],
        }
    }

    pub fn hashes(&self) -> &[SharedString] {
        &self.hashes
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
            let Ok(contents) = fs::read_to_string(&path) else {
                return;
            };

            let hashes: Vec<SharedString> = contents
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(SharedString::from)
                .collect();

            _ = this.update(cx, |this, cx| {
                this.path = Some(path);
                this.hashes = hashes;
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for ImsiHashFile {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = match (&self.path, self.hashes.len()) {
            (Some(path), count) => format!(
                "{} ({count} hashes)",
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ),
            (None, _) => "No file selected".to_string(),
        };

        div()
            .v_flex()
            .gap_2()
            .child(
                Button::new("select-imsi-hash-file")
                    .label("Select IMSI hash file...")
                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                        this.pick_file(cx);
                    })),
            )
            .child(div().w(px(260.)).child(status))
    }
}
