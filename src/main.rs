// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    action: Action,
}

#[derive(Debug, clap::Subcommand)]
enum Action {
    GenerateLogs(LogAction),
}

#[derive(Debug, Parser)]
pub struct LogAction {
    pub path: String,
}

fn main() {
    compio_runtime::Runtime::new().unwrap().block_on(async {
        let args = Args::parse();

        if let Err(why) = match args.action {
            Action::GenerateLogs(action) => generate_logs(&action.path).await,
        } {
            eprintln!("{:?}", why);
            std::process::exit(1);
        }
    })
}

async fn generate_logs(path: &str) -> anyhow::Result<()> {
    println!("PATH {}", pop_support::logs::generate(path).await?);
    Ok(())
}
