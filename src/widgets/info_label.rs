// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use super::{Description, InfoBox};
use gtk::prelude::*;
use relm::{Relm, Widget};

#[derive(Debug, Default)]
pub struct InfoLabelModel {
    description: String,
    value: String,
}

#[derive(Debug, relm_derive::Msg)]
pub enum InfoLabelEvent {
    SetLabel(String),
}

#[relm_derive::widget]
impl Widget for InfoLabel {
    fn model(_relm: &Relm<Self>, description: String) -> InfoLabelModel {
        InfoLabelModel {
            description,
            value: String::new(),
        }
    }

    fn update(&mut self, event: InfoLabelEvent) {
        let InfoLabelEvent::SetLabel(value) = event;
        self.model.value = value;
    }

    relm::view! {
        InfoBox {
            Description(self.model.description.clone()),

            gtk::Label {
                text: &self.model.value,
                halign: gtk::Align::End,
                valign: gtk::Align::Center,
            }
        }
    }
}
