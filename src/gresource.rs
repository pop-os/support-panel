// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

pub fn init() -> Result<(), glib::Error> {
    const GRESOURCE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/compiled.gresource"));

    gio::resources_register(&gio::Resource::from_data(&glib::Bytes::from_static(
        GRESOURCE,
    ))?);

    Ok(())
}
