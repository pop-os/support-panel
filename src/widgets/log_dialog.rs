// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use gtk::prelude::*;
use relm::Relm;

#[derive(Debug, Default)]
pub struct LogModel {
    dialog: Option<gtk::Dialog>,
    folder: Option<String>,
}

#[derive(relm_derive::Msg)]
pub enum LogEvent {
    Close,
    GeneratedLogs(anyhow::Result<String>),
    ShowInFolder,
}

#[relm_derive::widget]
impl relm::Widget for LogDialog {
    fn init_view(&mut self) {
        self.widgets.header.style_context().add_class("h1");
    }

    fn model(_: &Relm<Self>, dialog: gtk::Dialog) -> LogModel {
        LogModel {
            dialog: Some(dialog),
            folder: None,
        }
    }

    fn update(&mut self, event: LogEvent) {
        match event {
            LogEvent::GeneratedLogs(result) => {
                if let Ok(archive) = result {
                    let message = format!("A log archive ({archive}) was created.");

                    self.widgets.description.set_text(&message);

                    if let Some(pos) = archive.rfind('/') {
                        let folder = String::from(&archive[..pos]);
                        self.widgets.show_folder_button.set_sensitive(true);
                        self.widgets.close_button.set_sensitive(true);
                        self.model.folder = Some(folder);
                    }

                    return;
                }
            }

            LogEvent::Close => (),

            LogEvent::ShowInFolder => {
                if let Some(folder) = self.model.folder.take() {
                    let _ = async_process::Command::new("xdg-open").arg(&folder).spawn();
                }
            }
        }

        if let Some(dialog) = self.model.dialog.take() {
            dialog.close();
        }
    }

    relm::view! {
        gtk::Box {
            orientation: gtk::Orientation::Vertical,
            spacing: 24,
            margin_top: 24,

            #[name="header"]
            gtk::Label {
                label: "<b>Create Log Files</b>",
                halign: gtk::Align::Center,
                use_markup: true,
            },

            #[name="description"]
            gtk::Label {
                label: "Creating Files...",
                halign: gtk::Align::Center,
                line_wrap: true,
            },

            gtk::ButtonBox {
                hexpand: true,
                homogeneous: true,
                layout_style: gtk::ButtonBoxStyle::Expand,
                orientation: gtk::Orientation::Horizontal,
                valign: gtk::Align::End,

                #[name="close_button"]
                gtk::Button {
                    gtk::Label {
                        label: "Close",
                        margin_top: 8,
                        margin_bottom: 8,
                    },
                    sensitive: false,
                    clicked => LogEvent::Close,
                },

                #[name="show_folder_button"]
                gtk::Button {
                    gtk::Label {
                        label: "Show in Folder",
                        margin_top: 8,
                        margin_bottom: 8,
                    },
                    sensitive: false,
                    clicked => LogEvent::ShowInFolder
                }
            }
        }
    }
}
