use crate::vendor::Vendor;
use concat_in_place::strcat;
use smol::fs::read_to_string;
use std::process::Command;

use const_format::concatcp;

const DMI_DIR: &str = "/sys/devices/virtual/dmi/id/";
const BOARD_NAME: &str = concatcp!(DMI_DIR, "board_name");
const BOARD_VERSION: &str = concatcp!(DMI_DIR, "board_version");
const PRODUCT_NAME: &str = concatcp!(DMI_DIR, "product_name");
const PRODUCT_VERSION: &str = concatcp!(DMI_DIR, "product_version");
const SYS_VENDOR: &str = concatcp!(DMI_DIR, "sys_vendor");

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
        let vendor = Vendor::guess();

        let (dmi_name, dmi_version) = match vendor {
            Some(_) => (PRODUCT_NAME, PRODUCT_VERSION),
            None => (BOARD_NAME, BOARD_VERSION),
        };

        let (sys_vendor, version, product_name, os_release) = futures::join!(
            read_to_string(SYS_VENDOR),
            read_to_string(dmi_version),
            read_to_string(dmi_name),
            read_to_string("/etc/os-release"),
        );

        let mut model_and_version = String::new();

        if let Ok(mut sys_vendor) = sys_vendor.as_deref() {
            sys_vendor = sys_vendor.trim();

            model_and_version.clear();
            model_and_version.push_str(sys_vendor);

            if let Ok(mut name) = product_name.as_deref() {
                name = name.trim();

                if !name.is_empty() && name != sys_vendor  {
                    strcat!(&mut model_and_version, " " name.trim());
                }
            }

            if let Ok(mut version) = version.as_deref() {
                version = version.trim();
                if !version.is_empty() {
                    strcat!(&mut model_and_version, " (" version.trim() ")");
                }
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
            .next()
            .unwrap_or_default()
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
