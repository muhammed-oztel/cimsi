use gpui_kit::component::combobox::{ComboboxEvent, ComboboxState};
use gpui_kit::component::input::{Input, InputEvent, InputState, MaskPattern};
use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::StyledExt;
use gpui_kit::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    Subscription, Window, div, rgb,
};

use crate::imsi::{Operator, imsi_prefix_for};
use crate::imsi_search::{IMSI_TOTAL_LEN, Position, compose_pattern};

fn digits_mask(remaining: usize) -> MaskPattern {
    MaskPattern::new(&"9".repeat(remaining))
}

pub struct ImsiDisplay {
    operator_prefix: &'static str,
    position: Position,
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
        let position = Position::Prefix;

        let digits_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Digits")
                .mask_pattern(digits_mask(IMSI_TOTAL_LEN - operator_prefix.len()))
        });

        let display_state = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_value(compose_pattern(operator_prefix, "", position), window, cx);
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
                    s.set_value(compose_pattern(this.operator_prefix, "", this.position), window, cx);
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
                        compose_pattern(this.operator_prefix, &digits, this.position),
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

    /// The full 15-digit IMSI pattern, with `*` standing in for each unknown digit.
    pub fn pattern(&self, cx: &App) -> SharedString {
        self.display_state.read(cx).value()
    }

    /// Restore `operator_prefix`, known digits, and position directly, e.g.
    /// when resuming from a checkpoint (the country/operator comboboxes must
    /// be set separately — this doesn't go through their subscriptions).
    pub fn restore(
        &mut self,
        operator_prefix: &'static str,
        digits: &str,
        position: Position,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.operator_prefix = operator_prefix;
        self.position = position;

        self.digits_state.update(cx, |s, cx| {
            s.set_mask_pattern(digits_mask(IMSI_TOTAL_LEN - operator_prefix.len()), window, cx);
            s.set_value(digits, window, cx);
        });

        self.display_state.update(cx, |s, cx| {
            s.set_value(compose_pattern(operator_prefix, digits, position), window, cx);
        });

        cx.notify();
    }

    fn set_position(&mut self, position: Position, window: &mut Window, cx: &mut Context<Self>) {
        self.position = position;

        let digits = self.digits_state.read(cx).unmask_value();
        self.display_state.update(cx, |s, cx| {
            s.set_value(compose_pattern(self.operator_prefix, &digits, position), window, cx);
        });

        cx.notify();
    }
}

impl Render for ImsiDisplay {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_index = match self.position {
            Position::Prefix => 0,
            Position::Suffix => 1,
        };

        div()
            .v_flex()
            .gap_2()
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .child(div().text_xs().text_color(rgb(0x6c7484)).child("Position"))
                    .child(
                        RadioGroup::horizontal("imsi-position")
                            .child(Radio::new("prefix").label("Prefix"))
                            .child(Radio::new("suffix").label("Suffix"))
                            .selected_index(Some(selected_index))
                            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                                let position = if *ix == 0 {
                                    Position::Prefix
                                } else {
                                    Position::Suffix
                                };
                                this.set_position(position, window, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .w_full()
                    .child(div().text_xs().text_color(rgb(0x6c7484)).child("Known digits"))
                    .child(Input::new(&self.digits_state).w_full()),
            )
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .w_full()
                    .child(div().text_xs().text_color(rgb(0x6c7484)).child("Pattern"))
                    .child(Input::new(&self.display_state).disabled(true).w_full()),
            )
    }
}
