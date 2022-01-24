// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

#[macro_use]
extern crate cascade;

pub mod gresource;
pub mod system76;
pub mod widgets;

mod localize;
mod vendor;

pub use self::vendor::Vendor;

use self::widgets::*;
use anyhow::Context;
use concat_in_place::strcat;
use gtk::prelude::*;
use i18n_embed::DesktopLanguageRequester;
use relm::{Relm, Widget};

pub fn localize() {
    let localizer = localize::localizer();
    let requested_languages = DesktopLanguageRequester::requested_languages();

    if let Err(error) = localizer.select(&requested_languages) {
        eprintln!("{} failed to load langauge: {}", file!(), error);
    }
}

#[derive(Debug, relm_derive::Msg)]
pub enum SupportEvent {
    UpdateInfo(SupportInfo),
    BrowseDocumentation,
    CommunitySupport,
    CreateLogFiles,
    CreateSupportTicket,
}

pub struct SupportModel {
    vendor: Option<Vendor>,
    window: gtk::Window,
    log_dialog: Option<relm::Component<LogDialog>>,
}

#[derive(Debug, Default)]
pub struct SupportInfo {
    vendor: Option<Vendor>,
    model_and_version: String,
    serial_number: String,
    operating_system: String,
}

#[relm_derive::widget]
impl Widget for SupportPanel {
    fn init_view(&mut self) {
        self.widgets.settings_box.style_context().add_class("frame");

        self.widgets
            .settings_box
            .set_header_func(Some(Box::new(separator_header)));

        self.widgets
            .settings_box
            .set_selection_mode(gtk::SelectionMode::None);

        cascade! {
            gtk::SizeGroup::new(gtk::SizeGroupMode::Both);
            ..add_widget(&self.widgets.model_info);
            ..add_widget(&self.widgets.serial_info);
            ..add_widget(&self.widgets.os_info);
            ..add_widget(&self.widgets.box4);
            ..add_widget(&self.widgets.box5);
            ..add_widget(&self.widgets.box6);
            ..add_widget(&self.widgets.box7);
        };

        cascade! {
            gtk::SizeGroup::new(gtk::SizeGroupMode::Both);
            ..add_widget(&self.widgets.button1);
            ..add_widget(&self.widgets.button2);
            ..add_widget(&self.widgets.button3);
            ..add_widget(&self.widgets.button4);
        };
    }

    fn model(relm: &Relm<Self>, window: gtk::Window) -> SupportModel {
        let stream = relm.stream().clone();

        glib::MainContext::default().spawn_local(async move {
            use async_fs::read_to_string;

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

            if model_and_version.is_empty() {
                model_and_version = fl!("unknown")
            }

            if operating_system.is_empty() {
                operating_system = fl!("unknown");
            }

            stream.emit(SupportEvent::UpdateInfo(SupportInfo {
                vendor,
                model_and_version,
                serial_number: fl!("unknown"),
                operating_system,
            }));
        });

        SupportModel {
            vendor: None,
            window,
            log_dialog: None,
        }
    }

    fn update(&mut self, event: SupportEvent) {
        match event {
            SupportEvent::UpdateInfo(info) => {
                let serial_number_row = self.widgets.settings_box.row_at_index(1).unwrap();
                serial_number_row.show();

                self.components
                    .model_info
                    .emit(InfoLabelEvent::SetLabel(info.model_and_version));

                self.components
                    .serial_info
                    .emit(InfoLabelEvent::SetLabel(info.serial_number));

                self.components
                    .os_info
                    .emit(InfoLabelEvent::SetLabel(info.operating_system));

                self.model.vendor = info.vendor;

                serial_number_row.hide();

                let set_by_resource = |resource: &str| {
                    let pixbuf =
                        gdk_pixbuf::Pixbuf::from_resource_at_scale(resource, 256, 256, true);

                    if let Ok(pixbuf) = pixbuf {
                        self.widgets.support_logo.set_pixbuf(Some(&pixbuf));
                    }
                };

                let set_by_image = |image: &str| {
                    self.widgets.support_logo.set_icon_name(Some(image));
                    self.widgets.support_logo.set_size_request(256, 256);
                };

                if let Some(vendor) = info.vendor {
                    self.widgets.box6.show();

                    match vendor {
                        Vendor::System76 => set_by_resource("/org/pop/support/system76.png"),
                    }
                } else {
                    self.widgets.box6.hide();

                    set_by_image("distributor-logo-pop-os")
                }
            }

            SupportEvent::BrowseDocumentation => {
                open_url("https://support.system76.com");
            }

            SupportEvent::CommunitySupport => open_url("https://chat.pop-os.org"),

            SupportEvent::CreateSupportTicket => {
                open_url("https://system76.com/my-account/support-tickets/new")
            }

            SupportEvent::CreateLogFiles => {
                #[allow(clippy::single_match)]
                match self.model.vendor {
                    None | Some(Vendor::System76) => {
                        let dialog = gtk::DialogBuilder::new()
                            .transient_for(&self.model.window)
                            .modal(true)
                            .decorated(false)
                            .resizable(false)
                            .default_width(480)
                            .build();

                        let dialog_inner = relm::init::<LogDialog>(dialog.clone()).unwrap();

                        dialog.content_area().add(dialog_inner.widget());

                        dialog.show();

                        let stream = dialog_inner.stream();
                        let (_channel, sender) = relm::Channel::new(move |result| {
                            stream.emit(LogEvent::GeneratedLogs(result))
                        });

                        std::thread::spawn(move || {
                            let _ = sender.send(generate_logs_subprocess());
                        });

                        // Keeps the event stream alive for as long as the dialog needs it.
                        self.model.log_dialog = Some(dialog_inner);
                    }
                }
            }
        }
    }

    relm::view! {
        gtk::ScrolledWindow {
            hscrollbar_policy: gtk::PolicyType::Never,

            gtk::Box {
                halign: gtk::Align::Center,
                width_request: 600,
                orientation: gtk::Orientation::Vertical,

                #[name="support_logo"]
                gtk::Image {
                    margin_top: 48,
                    margin_bottom: 48
                },

                #[name="settings_box"]
                gtk::ListBox {
                    #[name="model_info"]
                    InfoLabel(fl!("model-and-version")),

                    #[name="serial_info"]
                    InfoLabel(fl!("serial-number")),

                    #[name="os_info"]
                    InfoLabel(fl!("os-version")),

                    #[name="box4"]
                    InfoBox {
                        Description(fl!("documentation")),

                        #[name="button1"]
                        gtk::Button {
                            label: &fl!("documentation-button"),
                            clicked => SupportEvent::BrowseDocumentation,
                        }
                    },

                    #[name="box5"]
                    InfoBox {
                        Description(fl!("support-community")),

                        #[name="button2"]
                        gtk::Button {
                            label: &fl!("support-community-button"),
                            clicked => SupportEvent::CommunitySupport,
                        }
                    },

                    #[name="box6"]
                    InfoBox {
                        Description(fl!("support-professional")),

                        #[name="button3"]
                        gtk::Button {
                            label: &fl!("support-professional-button"),
                            clicked => SupportEvent::CreateSupportTicket,
                        }
                    },

                    #[name="box7"]
                    InfoBox {
                        Description(fl!("create-logs")),

                        #[name="button4"]
                        gtk::Button {
                            label: &fl!("create-logs-button"),
                            clicked => SupportEvent::CreateLogFiles,
                        }
                    },
                }
            }
        }
    }
}

pub fn generate_logs_subprocess() -> anyhow::Result<String> {
    std::process::Command::new("pkexec")
        .arg("pop-support")
        .arg("generate-logs")
        .output()
        .context("failed to start command to generate logs")
        .and_then(|output| {
            let output = String::from_utf8(output.stdout)
                .context("output of command to generate logs is corrupted")?;

            let path = dbg!(&output)
                .strip_prefix("PATH ")
                .context("command that generated logs did not provide path to logs")?;

            Ok(path.trim().to_owned())
        })
}

fn open_url(url: &'static str) {
    std::thread::spawn(move || {
        let _ = std::process::Command::new("xdg-open").arg(url).status();
    });
}

fn separator_header(current: &gtk::ListBoxRow, before: Option<&gtk::ListBoxRow>) {
    if before.is_some() {
        current.set_header(Some(&gtk::Separator::new(gtk::Orientation::Horizontal)));
    }
}
