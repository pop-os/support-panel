// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

mod clamp;
mod info_box;
mod info_label;
mod log_dialog;

pub use self::clamp::*;
pub use self::info_box::*;
pub use self::info_label::*;
pub use self::log_dialog::*;

use gtk::prelude::*;

#[relm_derive::widget]
impl relm::Widget for Description {
    fn model(description: String) -> String {
        description
    }

    fn update(&mut self, _: ()) {}

    relm::view! {
        gtk::Label {
            text: &self.model,
            halign: gtk::Align::Start,
            hexpand: true,
            valign: gtk::Align::Center,
            ellipsize: gtk::pango::EllipsizeMode::End,
        }
    }
}
