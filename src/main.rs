use gpui_kit::component::button::*;
use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::component::*;
use gpui_kit::*;

use crate::components::brute_force::BruteForce;
use crate::components::checkbox::ControlledCheckbox;
use crate::components::country_combobox::CountryCombobox;
use crate::components::hash_options::HashOptions;
use crate::components::imsi_display::ImsiDisplay;
use crate::components::imsi_hash_file::ImsiHashFile;
use crate::components::operator_combobox::OperatorCombobox;

mod checkpoint;
mod cli;
mod components;
mod hash_file;
mod hashing;
mod imsi;
mod imsi_search;

pub struct HelloWorld {
    country: Entity<CountryCombobox>,
    operator: Entity<OperatorCombobox>,
    imsi: Entity<ImsiDisplay>,
    hash_file: Entity<ImsiHashFile>,
    hash_options: Entity<HashOptions>,
    brute_force: Entity<BruteForce>,
}

impl HelloWorld {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let country = cx.new(|cx| CountryCombobox::new(window, cx));
        let country_state = country.read(cx).state().clone();
        let operator = cx.new(|cx| OperatorCombobox::new(&country_state, window, cx));
        let operator_state = operator.read(cx).state().clone();
        let imsi = cx.new(|cx| ImsiDisplay::new(&operator_state, window, cx));
        let hash_file = cx.new(|cx| ImsiHashFile::new(window, cx));
        let hash_options = cx.new(|cx| HashOptions::new(window, cx));
        let brute_force = cx.new(|cx| {
            BruteForce::new(
                country.clone(),
                operator.clone(),
                imsi.clone(),
                hash_options.clone(),
                hash_file.clone(),
                window,
                cx,
            )
        });

        Self {
            country,
            operator,
            imsi,
            hash_file,
            hash_options,
            brute_force,
        }
    }
}

/// Bordered, titled group box used to cluster related fields (see the
/// "Target" / "Hash" / "Results" panels below) — plain `div`s stacked
/// top-to-bottom gave no visual separation between unrelated fields.
fn card(title: &'static str, content: impl IntoElement) -> impl IntoElement {
    div()
        .v_flex()
        .gap_3()
        .p_3()
        .bg(rgb(0x1b2436))
        .border_1()
        .border_color(rgb(0x2c3650))
        .rounded_md()
        .child(
            div()
                .h_flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .size(px(6.))
                        .rounded_full()
                        .bg(rgb(0x49c2d9)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6c7484))
                        .child(title.to_uppercase()),
                ),
        )
        .child(content)
}

/// A small caption above a single control, so a bare combobox/input isn't
/// left unlabeled — reused for every field across the Target/Hash panels.
fn field(label: &'static str, content: impl IntoElement) -> impl IntoElement {
    div()
        .v_flex()
        .gap_1()
        .w_full()
        .child(div().text_xs().text_color(rgb(0x6c7484)).child(label))
        .child(content)
}

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h_flex()
            .items_start()
            .gap_3()
            .size_full()
            .p_3()
            .child(
                div()
                    .v_flex()
                    .gap_3()
                    .w(px(340.))
                    .flex_shrink_0()
                    .child(card(
                        "Target",
                        div()
                            .v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .h_flex()
                                    .gap_2()
                                    .child(div().flex_1().child(field("Country", self.country.clone())))
                                    .child(div().flex_1().child(field("Operator", self.operator.clone()))),
                            )
                            .child(self.imsi.clone()),
                    ))
                    .child(card(
                        "Hash",
                        div()
                            .v_flex()
                            .gap_2()
                            .child(self.hash_options.clone())
                            .child(self.hash_file.clone()),
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .child(card("Results", self.brute_force.clone())),
            )
    }
}

/// Repaints the default gpui-kit dark theme with the "scanner console"
/// palette from the approved mockup (bg-void/base/surface/field, cyan
/// accent, red danger, green success) so built-in widgets (Combobox, Input,
/// Button, Progress, Radio) match the hand-styled cards/labels in
/// `HelloWorld::render` instead of clashing with them.
fn apply_theme(cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);

    let theme = Theme::global_mut(cx);

    let bg_base: Hsla = rgb(0x10131c).into();
    let bg_surface: Hsla = rgb(0x1b2436).into();
    let bg_surface_raised: Hsla = rgb(0x242f47).into();
    let bg_surface_hover: Hsla = rgb(0x2c3856).into();
    let bg_surface_active: Hsla = rgb(0x161d2c).into();
    let border: Hsla = rgb(0x2c3650).into();
    let text_primary: Hsla = rgb(0xe7eaef).into();
    let text_muted: Hsla = rgb(0x6c7484).into();
    let accent: Hsla = rgb(0x49c2d9).into();
    let accent_strong: Hsla = rgb(0x6fd6ea).into();
    let accent_dim: Hsla = rgb(0x1c3a40).into();
    let danger: Hsla = rgb(0xe0555a).into();
    let danger_hover: Hsla = rgb(0xef7377).into();
    let danger_active: Hsla = rgb(0xc84448).into();
    let success: Hsla = rgb(0x4bc98a).into();
    let warning: Hsla = rgb(0xd9a441).into();

    theme.background = bg_base;
    theme.foreground = text_primary;
    theme.border = border;
    theme.input = border;
    theme.muted = bg_surface_raised;
    theme.muted_foreground = text_muted;
    theme.popover = bg_surface;
    theme.popover_foreground = text_primary;

    theme.accent = bg_surface_raised;
    theme.accent_foreground = accent_strong;
    theme.ring = accent;
    theme.caret = accent;
    theme.selection = accent_dim;

    theme.primary = accent;
    theme.primary_foreground = bg_base;
    theme.primary_hover = accent_strong;
    theme.primary_active = accent;

    theme.secondary = bg_surface_raised;
    theme.secondary_foreground = text_primary;
    theme.secondary_hover = bg_surface_hover;
    theme.secondary_active = bg_surface_active;

    theme.button = bg_surface_raised;
    theme.button_foreground = text_primary;
    theme.button_hover = bg_surface_hover;
    theme.button_active = bg_surface_active;

    theme.danger = danger;
    theme.danger_foreground = rgb(0xffffff).into();
    theme.danger_hover = danger_hover;
    theme.danger_active = danger_active;

    theme.success = success;
    theme.success_foreground = bg_base;
    theme.warning = warning;
    theme.warning_foreground = bg_base;

    theme.progress_bar = accent_strong;

    theme.colors.list = bg_surface;
    theme.colors.list_hover = bg_surface_raised;
    theme.colors.list_active = accent_dim;
    theme.colors.list_active_border = accent;
    theme.colors.list_even = bg_surface;
    theme.colors.list_head = bg_surface;

    theme.scrollbar_thumb = border;
    theme.scrollbar_thumb_hover = bg_surface_hover;

    theme.window_border = border;

    // `Theme::change` is what normally recomputes `tokens` from `colors`
    // (via `apply_config`); mutating `colors` fields directly above leaves
    // the legacy `tokens` snapshot stale, and `Root` paints the window
    // background from `tokens.background`, not `colors.background` — so
    // without this the whole window keeps rendering the old near-black.
    theme.tokens = theme.colors.into();

    Theme::sync_base(cx);
}

fn main() -> std::process::ExitCode {
    use clap::Parser;

    let args = cli::Cli::parse();

    if let Some(command) = args.command {
        return cli::run(command);
    }

    run_gui();
    std::process::ExitCode::SUCCESS
}

fn run_gui() {
    let app = gpui_kit::application().with_assets(gpui_kit::assets::Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);
        apply_theme(cx);

        let bounds = Bounds::centered(None, size(px(900.), px(620.)), cx);
        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    is_resizable: true,
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
