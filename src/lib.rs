// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

#[macro_use]
extern crate cascade;

pub mod gresource;
pub mod support_info;
pub mod system76;
pub mod widgets;

mod localize;
mod vendor;

pub use self::vendor::Vendor;

use self::support_info::SupportInfo;
use self::widgets::*;
use anyhow::Context;
use gtk::prelude::*;
use i18n_embed::DesktopLanguageRequester;
use relm::{Relm, Widget};

const LOGO_SIZE: i32 = 256;

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

        let clamp = self.widgets.clamp.downgrade();

        let id = std::rc::Rc::new(std::cell::RefCell::new(None));

        self.widgets
            .root
            .connect_size_allocate(move |widget, _size| {
                if id.borrow().is_none() {
                    let clamp = clamp.clone();
                    let widget = widget.downgrade();
                    let id_ = id.clone();

                    use glib::timeout_add_local;
                    use std::time::Duration;

                    *id.borrow_mut() =
                        Some(timeout_add_local(Duration::from_millis(16), move || {
                            if let Some(widget) = widget.upgrade() {
                                if !widget.get_visible() {
                                    return glib::Continue(false);
                                }

                                let width = widget.allocation().width;
                                if width < 2 {
                                    return glib::Continue(true);
                                }

                                if let Some(clamp) = clamp.upgrade() {
                                    clamp.set_width_request(if width > 600 {
                                        600
                                    } else {
                                        (width * 90) / 100
                                    });
                                }
                            }

                            id_.borrow_mut().take();

                            glib::Continue(false)
                        }));
                }
            });
    }

    fn model(relm: &Relm<Self>, window: gtk::Window) -> SupportModel {
        let stream = relm.stream().clone();

        glib::MainContext::default().spawn_local(async move {
            let mut info = SupportInfo::fetch().await;

            if info.model_and_version.is_empty() {
                info.model_and_version = fl!("unknown")
            }

            if info.operating_system.is_empty() {
                info.operating_system = fl!("unknown");
            }

            if info.serial_number.is_empty() {
                info.serial_number = fl!("unknown");
            }

            stream.emit(SupportEvent::UpdateInfo(info));
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
                    let pixbuf = gdk_pixbuf::Pixbuf::from_resource_at_scale(
                        resource, LOGO_SIZE, LOGO_SIZE, true,
                    );

                    if let Ok(pixbuf) = pixbuf {
                        self.widgets.support_logo.set_pixbuf(Some(&pixbuf));
                    }
                };

                let set_by_image = |image: &str| {
                    self.widgets.support_logo.set_icon_name(Some(image));
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
        #[name="root"]
        gtk::ScrolledWindow {
            hscrollbar_policy: gtk::PolicyType::Never,

            #[name="clamp"]
            gtk::Box {
                halign: gtk::Align::Center,
                orientation: gtk::Orientation::Vertical,

                #[name="support_logo"]
                gtk::Image {
                    margin_top: 48,
                    margin_bottom: 48,
                    width_request: LOGO_SIZE,
                    height_request: LOGO_SIZE,
                    pixel_size: LOGO_SIZE,
                },

                #[name="settings_box"]
                gtk::ListBox {
                    margin_bottom: 48,

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
    let home_dir = dirs::home_dir().context("no home directory")?;

    std::process::Command::new("pkexec")
        .arg("pop-support")
        .arg("generate-logs")
        .arg(home_dir)
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
