use gpui_kit::component::button::*;
use gpui_kit::component::*;
use gpui_kit::*;

use crate::components::checkbox::ControlledCheckbox;
use crate::components::country_combobox::CountryCombobox;

mod components;

pub struct HelloWorld {
    terms: Entity<ControlledCheckbox>,
    country: Entity<CountryCombobox>,
}

impl HelloWorld {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            terms: cx.new(|_| ControlledCheckbox::new()),
            country: cx.new(|cx| CountryCombobox::new(window, cx)),
        }
    }
}

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let code = self.country.read(cx).selected_country(cx).unwrap_or("none");
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
            .child(self.country.clone())
            .child(format!("Selected country code: {code}"))
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
                    let view = cx.new(|cx| HelloWorld::new(window, cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}
