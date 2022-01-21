// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use anyhow::Context;
use clap::Parser;
use gtk::prelude::*;
use pop_support::SupportPanel;

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    action: Action,
}

#[derive(Debug, clap::Subcommand)]
enum Action {
    GenerateLogs,
    // Gtk,
}

fn main() {
    let args = Args::parse();

    if let Err(why) = match args.action {
        Action::GenerateLogs => generate_logs(),
        // Action::Gtk => gtk(),
    } {
        eprintln!("{:?}", why);
        std::process::exit(1);
    }
}

fn generate_logs() -> anyhow::Result<()> {
    use futures::executor::block_on;
    use pop_support::{system76, Vendor};

    let sys_vendor = std::fs::read_to_string("/sys/devices/virtual/dmi/id/sys_vendor")
        .context("could not fetch system vendor info")?;

    #[allow(clippy::single_match)]
    match Vendor::guess_from(sys_vendor.trim()) {
        Some(Vendor::System76) => {
            let path = block_on(system76::generate_logs())?;
            println!("PATH {path}");
        }

        _ => (),
    }

    Ok(())
}

#[allow(unused)]
fn gtk() -> anyhow::Result<()> {
    let _ = gtk::init();
    let _ = pop_support::gresource::init();

    let window = gtk::Window::new(gtk::WindowType::Toplevel);

    let panel = relm::init::<SupportPanel>(window.clone()).unwrap();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        gtk::Inhibit(false)
    });

    window.add(panel.widget());
    window.show_all();

    gtk::main();

    Ok(())
}
