// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use gtk::prelude::*;

#[relm_derive::widget]
impl relm::Widget for InfoBox {
    fn model(_: ()) {}

    fn update(&mut self, _: ()) {}

    relm::view! {
        #[container]
        gtk::Box {
            orientation: gtk::Orientation::Horizontal,
            margin_start: 20,
            margin_end: 20,
            margin_top: 8,
            margin_bottom: 8,
            spacing: 24
        }
    }
}
