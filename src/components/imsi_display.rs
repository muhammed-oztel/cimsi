use gpui_kit::component::combobox::{ComboboxEvent, ComboboxState};
use gpui_kit::component::input::{Input, InputEvent, InputState, MaskPattern};
use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::StyledExt;
use gpui_kit::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window,
    div, px,
};

use crate::components::operator_combobox::{Operator, imsi_prefix_for};

const IMSI_TOTAL_LEN: usize = 15;

#[derive(Clone, Copy, PartialEq)]
enum ImsiPosition {
    Prefix,
    Suffix,
}

fn digits_mask(remaining: usize) -> MaskPattern {
    MaskPattern::new(&"9".repeat(remaining))
}

fn compose_imsi(prefix: &str, digits: &str, position: ImsiPosition) -> String {
    let remaining = IMSI_TOTAL_LEN.saturating_sub(prefix.len());
    let stars = "*".repeat(remaining.saturating_sub(digits.len()));

    match position {
        ImsiPosition::Prefix => format!("{prefix}{digits}{stars}"),
        ImsiPosition::Suffix => format!("{prefix}{stars}{digits}"),
    }
}

pub struct ImsiDisplay {
    operator_prefix: &'static str,
    position: ImsiPosition,
    digits_state: Entity<InputState>,
    display_state: Entity<InputState>,
    _operator_subscription: Subscription,
    _digits_subscription: Subscription,
}

impl ImsiDisplay {
    pub fn new(
        operator_state: &Entity<ComboboxState<SearchableVec<Operator>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let operator_prefix = operator_state
            .read(cx)
            .selected_value()
            .and_then(imsi_prefix_for)
            .unwrap_or_default();
        let position = ImsiPosition::Prefix;

        let digits_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Digits")
                .mask_pattern(digits_mask(IMSI_TOTAL_LEN - operator_prefix.len()))
        });

        let display_state = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_value(compose_imsi(operator_prefix, "", position), window, cx);
            s
        });

        let _operator_subscription = cx.subscribe_in(operator_state, window, {
            let digits_state = digits_state.clone();
            let display_state = display_state.clone();
            move |this, _operator_state, event: &ComboboxEvent<SearchableVec<Operator>>, window, cx| {
                let (ComboboxEvent::Change(values) | ComboboxEvent::Confirm(values)) = event;

                this.operator_prefix = values
                    .first()
                    .copied()
                    .and_then(imsi_prefix_for)
                    .unwrap_or_default();

                digits_state.update(cx, |s, cx| {
                    s.set_mask_pattern(
                        digits_mask(IMSI_TOTAL_LEN - this.operator_prefix.len()),
                        window,
                        cx,
                    );
                    s.set_value("", window, cx);
                });

                display_state.update(cx, |s, cx| {
                    s.set_value(compose_imsi(this.operator_prefix, "", this.position), window, cx);
                });
            }
        });

        let _digits_subscription = cx.subscribe_in(&digits_state, window, {
            let display_state = display_state.clone();
            move |this, digits, event: &InputEvent, window, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }

                let digits = digits.read(cx).unmask_value();

                display_state.update(cx, |s, cx| {
                    s.set_value(
                        compose_imsi(this.operator_prefix, &digits, this.position),
                        window,
                        cx,
                    );
                });
            }
        });

        Self {
            operator_prefix,
            position,
            digits_state,
            display_state,
            _operator_subscription,
            _digits_subscription,
        }
    }

    fn set_position(&mut self, position: ImsiPosition, window: &mut Window, cx: &mut Context<Self>) {
        self.position = position;

        let digits = self.digits_state.read(cx).unmask_value();
        self.display_state.update(cx, |s, cx| {
            s.set_value(compose_imsi(self.operator_prefix, &digits, position), window, cx);
        });

        cx.notify();
    }
}

impl Render for ImsiDisplay {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_index = match self.position {
            ImsiPosition::Prefix => 0,
            ImsiPosition::Suffix => 1,
        };

        div()
            .v_flex()
            .gap_2()
            .child(
                RadioGroup::horizontal("imsi-position")
                    .child(Radio::new("prefix").label("Prefix"))
                    .child(Radio::new("suffix").label("Suffix"))
                    .selected_index(Some(selected_index))
                    .on_click(cx.listener(|this, ix: &usize, window, cx| {
                        let position = if *ix == 0 {
                            ImsiPosition::Prefix
                        } else {
                            ImsiPosition::Suffix
                        };
                        this.set_position(position, window, cx);
                    })),
            )
            .child(div().w(px(200.)).child(Input::new(&self.digits_state)))
            .child(
                div()
                    .w(px(200.))
                    .child(Input::new(&self.display_state).disabled(true)),
            )
    }
}
