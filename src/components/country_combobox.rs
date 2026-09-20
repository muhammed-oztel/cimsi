use std::vec;

use gpui_kit::base::IndexPath;
use gpui_kit::component::combobox::{
    Combobox, ComboboxEvent, ComboboxState, ComboboxTriggerContext,
};
use gpui_kit::component::searchable_list::{SearchableGroup, SearchableListItem, SearchableVec};
use gpui_kit::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div, px,
};

use crate::imsi::{COUNTRIES, Country};

impl SearchableListItem for Country {
    type Value = &'static str;

    fn title(&self) -> gpui_kit::SharedString {
        self.name.into()
    }

    fn value(&self) -> &Self::Value {
        &self.code
    }
}

pub struct CountryCombobox {
    state: Entity<ComboboxState<SearchableVec<Country>>>,
}

impl CountryCombobox {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| {
            let mut s =
                ComboboxState::new(SearchableVec::new(COUNTRIES.to_vec()), vec![], window, cx)
                    .searchable(true);

            s.set_selected_values(&["TUR"], window, cx);
            s
        });
        Self { state }
    }

    pub fn selected_country(&self, cx: &App) -> Option<&'static str> {
        self.state.read(cx).selected_value()
    }

    pub fn state(&self) -> &Entity<ComboboxState<SearchableVec<Country>>> {
        &self.state
    }
}

impl Render for CountryCombobox {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(200.)).child(
            Combobox::new(&self.state)
                .placeholder("Select the country of GSM Operator...")
                .w_full(),
        )
    }
}
