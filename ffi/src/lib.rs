// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use glib::translate::FromGlibPtrNone;
use gtk::prelude::{ContainerExt, WidgetExt};
use gtk_sys::{GtkContainer, GtkWindow};
use pop_support::{gresource, SupportPanel};

#[no_mangle]
pub extern "C" fn pop_support_attach(c_container: *mut GtkContainer, c_window: *mut GtkWindow) {
    let container;
    let window;

    unsafe {
        gtk::set_initialized();
        container = gtk::Container::from_glib_none(c_container);
        window = gtk::Window::from_glib_none(c_window);
    };

    let component = relm::init::<SupportPanel>(window).unwrap();

    let panel_widget = component.widget().clone();

    container.add(&panel_widget);

    // Take ownership of the panel to keep it alive until the container is destroyed.
    container.connect_destroy(move |_| {
        let _relm_handle = &component;
    });
}

#[no_mangle]
pub extern "C" fn pop_support_init() {
    let _ = gresource::init();
    pop_support::localize();
}
