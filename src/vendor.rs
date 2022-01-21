// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

#[derive(Copy, Clone, Debug)]
pub enum Vendor {
    System76,
}

impl Vendor {
    pub fn guess_from(sys_vendor: &str) -> Option<Self> {
        if "System76" == sys_vendor {
            return Some(Vendor::System76);
        }

        None
    }
}
