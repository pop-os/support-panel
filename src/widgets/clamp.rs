// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use gtk::prelude::*;

pub trait BinClamp {
    fn bin_clamp(&self, min: i32, max: i32, percent: i32);
}

impl<T> BinClamp for T
where
    T: IsA<gtk::Bin> + IsA<gtk::Widget>,
{
    fn bin_clamp(&self, min: i32, max: i32, percent: i32) {
        let id = std::rc::Rc::new(std::cell::RefCell::new(None));

        self.connect_size_allocate(move |widget, _size| {
            if id.borrow().is_none() {
                let widget = widget.downgrade();
                let id_ = id.clone();

                use glib::timeout_add_local;
                use std::time::Duration;

                *id.borrow_mut() = Some(timeout_add_local(Duration::from_millis(16), move || {
                    if let Some(widget) = widget.upgrade() {
                        if !widget.get_visible() {
                            return glib::Continue(false);
                        }

                        let width = widget.allocation().width;
                        if width < 2 {
                            return glib::Continue(true);
                        }

                        if let Some(clamp) = widget.child() {
                            let width = max.min((width * percent) / 100).max(min);
                            clamp.set_width_request(width);

                            if let Some(viewport) = clamp.downcast_ref::<gtk::Viewport>() {
                                if let Some(child) = viewport.child() {
                                    child.set_width_request(width);
                                }
                            }
                        }
                    }

                    id_.borrow_mut().take();

                    glib::Continue(false)
                }));
            }
        });
    }
}
