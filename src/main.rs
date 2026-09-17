use gpui::{
    App, Bounds, Context, PathPromptOptions, SharedString, Window, WindowBounds, WindowOptions,
    div, prelude::*, px, rgb, size,
};

struct HelloWorld {
    text: SharedString,
    clicks: usize,
    file_path: Option<String>,
}

// impl Render for ByeWold {
//     fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//         div()
//             .flex()
//             .flex_col()
//             .gap_3()
//             .bg(rgb(0xffffff))
//             .size(px(300.0))
//             .justify_center()
//     }
// }

impl Render for HelloWorld {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0xffffff))
            .size(px(500.0))
            .justify_center()
            .items_center()
            .shadow_lg()
            .border_1()
            .border_color(rgb(0x0000ff))
            .text_xl()
            .text_color(rgb(0xffffff))
            .child(format!("Hello, {}!", &self.text))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(div().size_8().bg(gpui::red()))
                    .child(div().size_8().bg(gpui::green()))
                    .child(div().size_8().bg(gpui::blue()))
                    .child(div().size_8().bg(gpui::yellow()))
                    .child(div().size_8().bg(gpui::black()))
                    .child(div().size_8().bg(gpui::white())),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .text_3xl()
                    .text_color(rgb(0x456123))
                    .child("This is custom duude"),
            )
            .child(
                div()
                    .id("increment-button")
                    .px_4()
                    .py_2()
                    .bg(rgb(0x3b82f6))
                    .text_color(rgb(0xffffff))
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0x2563eb)))
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.clicks += 1;
                        cx.notify();
                    }))
                    .child(format!("Clicked {} times", self.clicks)),
            )
            .child(
                div()
                    .id("open-file-button")
                    .px_4()
                    .py_2()
                    .bg(rgb(0x16a34a))
                    .text_color(rgb(0xffffff))
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0x15803d)))
                    .on_click(cx.listener(|_this, _event, _window, cx| {
                        let paths = cx.prompt_for_paths(PathPromptOptions {
                            files: true,
                            directories: false,
                            multiple: false,
                            prompt: None,
                        });
                        cx.spawn(async move |this, cx| {
                            if let Ok(Ok(Some(mut paths))) = paths.await {
                                if let Some(path) = paths.pop() {
                                    this.update(cx, |this, cx| {
                                        this.file_path = Some(path.display().to_string());
                                        cx.notify();
                                    })
                                    .ok();
                                }
                            }
                        })
                        .detach();
                    }))
                    .child("Open File..."),
            )
            .child(format!(
                "Selected: {}",
                self.file_path.as_deref().unwrap_or("none")
            ))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(500.), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| HelloWorld {
                    text: "World".into(),
                    clicks: 0,
                    file_path: None,
                })
            },
        )
        .unwrap();
    });
}
