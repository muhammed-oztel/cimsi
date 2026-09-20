use std::vec;

use gpui_kit::component::combobox::{Combobox, ComboboxState};
use gpui_kit::component::input::{Input, InputState, MaskPattern};
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::StyledExt;
use gpui_kit::{App, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div, px};

const ALGORITHMS: &[&str] = &["MD5", "SHA-1", "SHA-256"];
const ENCODINGS: &[&str] = &["Hex", "Base64"];

pub struct HashOptions {
    algorithm_state: Entity<ComboboxState<SearchableVec<&'static str>>>,
    encoding_state: Entity<ComboboxState<SearchableVec<&'static str>>>,
    threads_state: Entity<InputState>,
}

impl HashOptions {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let algorithm_state = cx.new(|cx| {
            let mut s = ComboboxState::new(SearchableVec::new(ALGORITHMS.to_vec()), vec![], window, cx);
            s.set_selected_values(&[ALGORITHMS[0]], window, cx);
            s
        });

        let encoding_state = cx.new(|cx| {
            let mut s = ComboboxState::new(SearchableVec::new(ENCODINGS.to_vec()), vec![], window, cx);
            s.set_selected_values(&[ENCODINGS[0]], window, cx);
            s
        });

        let threads_state = cx.new(|cx| {
            let mut s = InputState::new(window, cx)
                .placeholder("Threads")
                .mask_pattern(MaskPattern::new("999"));
            s.set_value("1", window, cx);
            s
        });

        Self {
            algorithm_state,
            encoding_state,
            threads_state,
        }
    }

    pub fn algorithm(&self, cx: &App) -> &'static str {
        self.algorithm_state.read(cx).selected_value().unwrap_or(ALGORITHMS[0])
    }

    pub fn encoding(&self, cx: &App) -> &'static str {
        self.encoding_state.read(cx).selected_value().unwrap_or(ENCODINGS[0])
    }

    /// Number of threads to brute force with, parsed from the input and
    /// clamped to at least 1 (an empty or non-numeric field defaults to 1).
    pub fn threads(&self, cx: &App) -> usize {
        self.threads_state
            .read(cx)
            .unmask_value()
            .parse::<usize>()
            .unwrap_or(1)
            .max(1)
    }
}

impl Render for HashOptions {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h_flex()
            .gap_2()
            .child(
                div().w(px(140.)).child(
                    Combobox::new(&self.algorithm_state)
                        .placeholder("Algorithm")
                        .w_full(),
                ),
            )
            .child(
                div().w(px(140.)).child(
                    Combobox::new(&self.encoding_state)
                        .placeholder("Encoding")
                        .w_full(),
                ),
            )
            .child(div().w(px(80.)).child(Input::new(&self.threads_state)))
    }
}
