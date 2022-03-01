use crate::vendor::Vendor;
use concat_in_place::strcat;
use smol::fs::read_to_string;
use std::process::Command;

#[derive(Debug, Default)]
pub struct SupportInfo {
    pub vendor: Option<Vendor>,
    pub model_and_version: String,
    pub serial_number: String,
    pub operating_system: String,
    pub kernel_version: String,
    pub kernel_revision: String,
}

impl SupportInfo {
    pub async fn fetch() -> Self {
        let (sys_vendor, version, product_name, os_release) = futures::join!(
            read_to_string("/sys/devices/virtual/dmi/id/sys_vendor"),
            read_to_string("/sys/devices/virtual/dmi/id/product_version"),
            read_to_string("/sys/devices/virtual/dmi/id/product_name"),
            read_to_string("/etc/os-release"),
        );

        let mut model_and_version = String::new();

        let mut vendor = None;

        if let Ok(sys_vendor) = sys_vendor {
            vendor = Vendor::guess_from(sys_vendor.trim());

            model_and_version.clear();
            model_and_version.push_str(sys_vendor.trim());

            if let Ok(name) = product_name {
                strcat!(&mut model_and_version, " " name.trim());
            }

            if let Ok(version) = version {
                strcat!(&mut model_and_version, " (" version.trim() ")");
            }
        }

        let mut operating_system = String::new();

        if let Ok(os_release) = os_release {
            for line in os_release.lines() {
                if let Some(mut value) = line.strip_prefix("PRETTY_NAME=") {
                    if let Some(v) = value.strip_prefix('"') {
                        value = v;
                    }

                    if let Some(v) = value.strip_suffix('"') {
                        value = v;
                    }

                    operating_system.clear();
                    operating_system.push_str(value.trim());
                    break;
                }
            }
        }
        
        let uname_r = Command::new("uname")
            .arg("-r")
            .output()
            .expect("failed to get kernel release");
        let kernel_version = String::from_utf8_lossy(&uname_r.stdout)
            .to_string()
            .trim()
            .to_string();
        
        let uname_v = Command::new("uname")
            .arg("-v")
            .output()
            .expect("failed to get kernel version");
        let kernel_revision = String::from_utf8_lossy(&uname_v.stdout)
            .to_string()
            .split_whitespace()
            .nth(0)
            .unwrap_or("")
            .to_string();

        let serial_number = String::new();

        Self {
            model_and_version,
            operating_system,
            serial_number,
            vendor,
            kernel_version,
            kernel_revision,
        }
    }
}
