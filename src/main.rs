use gpui_kit::component::button::*;
use gpui_kit::component::*;
use gpui_kit::*;

use crate::components::checkbox::ControlledCheckbox;

mod components;

pub struct HelloWorld {
    terms: Entity<ControlledCheckbox>,
}

impl HelloWorld {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            terms: cx.new(|_| ControlledCheckbox::new()),
        }
    }
}

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
            .child(self.terms.clone())
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(gpui_kit::assets::Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);

        let bounds = Bounds::centered(None, size(px(400.), px(300.)), cx);
        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    is_resizable: false, // user can't resize or maximize
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| HelloWorld::new(cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}
