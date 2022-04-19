// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use std::fs::read_to_string;

#[derive(Copy, Clone, Debug)]
pub enum Vendor {
    Hp,
    System76,
}

impl Vendor {
    pub fn guess() -> Option<Self> {
        if let Ok(sys_vendor) = read_to_string("/sys/devices/virtual/dmi/id/sys_vendor") {
            #[allow(clippy::single_match)]
            match sys_vendor.trim() {
                "HP" => {
                    if let Ok(version) =
                        read_to_string("/sys/devices/virtual/dmi/id/board_name")
                    {
                        match version.trim() {
                            "8A78" => return Some(Vendor::Hp),
                            _ => (),
                        }
                    }
                }
                "System76" => return Some(Vendor::System76),
                _ => (),
            }
        }

        None
    }
}
