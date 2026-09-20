use std::vec;

use gpui_kit::component::combobox::{Combobox, ComboboxEvent, ComboboxState};
use gpui_kit::component::searchable_list::{SearchableListItem, SearchableVec};
use gpui_kit::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString,
    Styled, Subscription, Window, div,
};

use crate::imsi::{Country, Operator, operators_for};

impl SearchableListItem for Operator {
    type Value = &'static str;

    fn title(&self) -> SharedString {
        self.name.into()
    }

    fn value(&self) -> &Self::Value {
        &self.code
    }
}

pub struct OperatorCombobox {
    state: Entity<ComboboxState<SearchableVec<Operator>>>,
    _country_subscription: Subscription,
}

impl OperatorCombobox {
    pub fn new(
        country_state: &Entity<ComboboxState<SearchableVec<Country>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let initial_country = country_state.read(cx).selected_value().unwrap_or_default();

        let state = cx.new(|cx| {
            ComboboxState::new(
                SearchableVec::new(operators_for(initial_country)),
                vec![],
                window,
                cx,
            )
            .searchable(true)
        });

        let _country_subscription = cx.subscribe_in(country_state, window, {
            let state = state.clone();
            move |_this, _country_state, event: &ComboboxEvent<SearchableVec<Country>>, window, cx| {
                let ComboboxEvent::Change(values) = event else {
                    return;
                };

                let Some(country_code) = values.first().copied() else {
                    return;
                };

                state.update(cx, |op_state, cx| {
                    op_state.set_items(SearchableVec::new(operators_for(country_code)), window, cx);
                    op_state.clear_selection(cx);
                });
            }
        });

        Self {
            state,
            _country_subscription,
        }
    }

    pub fn selected_operator(&self, cx: &App) -> Option<&'static str> {
        self.state.read(cx).selected_value()
    }

    pub fn state(&self) -> &Entity<ComboboxState<SearchableVec<Operator>>> {
        &self.state
    }

    /// Refresh the item list for `country_code` and select `operator_code`
    /// directly, e.g. when restoring from a checkpoint. Purely cosmetic —
    /// programmatic selection doesn't emit a `ComboboxEvent`, so it won't
    /// cascade into `ImsiDisplay`.
    pub fn set_selected(
        &mut self,
        country_code: &str,
        operator_code: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.update(cx, |s, cx| {
            s.set_items(SearchableVec::new(operators_for(country_code)), window, cx);
            s.set_selected_values(&[operator_code], window, cx);
        });
    }
}

impl Render for OperatorCombobox {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w_full().child(
            Combobox::new(&self.state)
                .placeholder("Select operator...")
                .w_full(),
        )
    }
}
