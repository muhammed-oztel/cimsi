use std::vec;

use gpui_kit::component::combobox::{Combobox, ComboboxEvent, ComboboxState};
use gpui_kit::component::searchable_list::{SearchableListItem, SearchableVec};
use gpui_kit::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString,
    Styled, Subscription, Window, div, px,
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
}

impl Render for OperatorCombobox {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(200.)).child(
            Combobox::new(&self.state)
                .placeholder("Select the GSM operator...")
                .w_full(),
        )
    }
}
