// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use anyhow::Context;
use as_result::IntoResult;
use async_fs::File as AsyncFile;
use async_process::{Command, Stdio};
use std::{fs::File, path::Path};

pub async fn generate_logs() -> anyhow::Result<&'static str> {
    let tempdir = tempfile::tempdir().context("failed to fetch temporary directory")?;
    let temppath = tempdir.path();

    let apt_history = &temppath.join("apt_history");
    let xorg_log = &temppath.join("Xorg.0.log");
    let syslog = &temppath.join("syslog");

    const LOG_PATH: &str = "/tmp/pop-support/system76-logs.tar.xz";

    let _ = std::fs::create_dir_all("/tmp/pop-support/");

    futures::try_join!(
        dmidecode(tempfile(temppath, "dmidecode")?),
        lspci(tempfile(temppath, "lspci")?),
        lsusb(tempfile(temppath, "lsusb")?),
        dmesg(tempfile(temppath, "dmesg")?),
        journalctl(tempfile(temppath, "journalctl")?),
        upower(tempfile(temppath, "upower")?),
        copy(Path::new("/var/log/apt/history.log"), apt_history),
        copy(Path::new("/var/log/Xorg.0.log"), xorg_log),
        copy(Path::new("/var/log/syslog"), syslog),
    )?;

    Command::new("tar")
        .arg("-C")
        .arg(temppath)
        .arg("-Jpcf")
        .arg(LOG_PATH)
        .args(&[
            "apt_history",
            "dmesg",
            "dmidecode",
            "journalctl",
            "lspci",
            "lsusb",
            "syslog",
            "upower",
            "Xorg.0.log",
        ])
        .status()
        .await
        .and_then(IntoResult::into_result)
        .context("tar exited in failure")?;

    Ok(LOG_PATH)
}

async fn command(command: &str, args: &[&str], output: File) -> anyhow::Result<()> {
    eprintln!("fetching output from `{} {}`", command, args.join(" "));
    Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(output)
        .status()
        .await
        .and_then(IntoResult::into_result)
        .with_context(|| format!("{} exited in failure", command))
}

async fn copy(source: &Path, dest: &Path) -> anyhow::Result<()> {
    eprintln!("copying logs from {}", source.display());
    let source = async move {
        AsyncFile::open(source)
            .await
            .context("failed to open source")
    };

    let dest = async move {
        AsyncFile::create(dest)
            .await
            .context("failed to create dest")
    };

    let (mut source, mut dest) = futures::try_join!(source, dest)?;
    futures::io::copy(&mut source, &mut dest)
        .await
        .context("failed to copy")
        .map(|_| ())
}

async fn dmesg(file: File) -> anyhow::Result<()> {
    command("dmesg", &[], file).await
}

async fn dmidecode(file: File) -> anyhow::Result<()> {
    command("dmidecode", &[], file).await
}

async fn journalctl(file: File) -> anyhow::Result<()> {
    command("journalctl", &["--since", "yesterday"], file).await
}

async fn lspci(file: File) -> anyhow::Result<()> {
    command("lspci", &["-vv"], file).await
}

async fn lsusb(file: File) -> anyhow::Result<()> {
    command("lsusb", &["-vv"], file).await
}

async fn upower(file: File) -> anyhow::Result<()> {
    command("upower", &["-d"], file).await
}

fn tempfile(path: &Path, command: &str) -> anyhow::Result<File> {
    File::create(path.join(command))
        .with_context(|| format!("failed to create temporary file for {}", command))
}
