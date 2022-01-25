// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use anyhow::Context;
use as_result::IntoResult;
use std::ffi::OsStr;
use std::{fs::File, path::Path, process::Stdio};
use tokio::fs::File as AsyncFile;
use tokio::process::Command;

pub async fn generate_logs(home: &str) -> anyhow::Result<String> {
    let tempdir = tempfile::tempdir().context("failed to fetch temporary directory")?;

    async fn system_info(file: File) -> anyhow::Result<()> {
        use tokio::io::AsyncWriteExt;

        let info = crate::support_info::SupportInfo::fetch().await;

        let data = fomat_macros::fomat! {
            "System76 Model: " (info.model_and_version) "\n"
            "OS Version: " (info.operating_system) "\n"
            "Kernel Version: " (info.kernel) "\n"
        };

        AsyncFile::from(file)
            .write_all(data.as_bytes())
            .await
            .context("failed to write system info")
    }

    let temp = tempdir.path();

    let _ = futures::join!(
        command("df", &["-h"], temp),
        command("dmesg", &[], temp),
        command("dmidecode", &[], temp),
        command("journalctl", &["--since", "yesterday"], temp),
        command("lsblk", &["-f"], temp),
        command("lspci", &["-vv"], temp),
        command("lsusb", &["-vv"], temp),
        command("sensors", &[], temp),
        command("upower", &["-d"], temp),
        command("uptime", &[], temp),
        copy(temp, "/etc/apt/sources.list.d", "apt/sources.list.d"),
        copy(temp, "/etc/apt/sources.list", "apt/sources.list"),
        copy(temp, "/etc/fstab", "fstab"),
        copy(temp, "/var/log/apt/history.log", "apt/history.log"),
        copy(
            temp,
            "/var/log/apt/history.log.1.gz",
            "apt/history-rotated.log"
        ),
        copy(temp, "/var/log/apt/term.log", "apt/term.log"),
        copy(temp, "/var/log/apt/term.log.1.gz", "apt/term-rotated.log"),
        copy(temp, "/var/log/syslog", "syslog.log"),
        copy(temp, "/var/log/Xorg.0.log", "Xorg.0.log"),
        system_info(tempfile(temp, "systeminfo.txt")?),
    );

    let files_to_collect: Vec<String> = std::fs::read_dir(temp)
        .map(|dir| {
            dir.filter_map(Result::ok)
                .filter_map(|entry| Some(entry.file_name().to_str()?.to_owned()))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    eprintln!("logs generated: {:?}", files_to_collect);

    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let log_path = format!("{home}/pop-support_{:?}.tar.xz", time);

    Command::new("tar")
        .arg("-C")
        .arg(temp)
        .arg("-Jpcf")
        .arg(&log_path)
        .args(&files_to_collect)
        .status()
        .await
        .and_then(IntoResult::into_result)
        .context("tar exited in failure")?;

    Ok(log_path)
}

async fn command(command: &str, args: &[&str], temp: &Path) -> anyhow::Result<()> {
    eprintln!("fetching output from `{command}`");
    Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(tempfile(temp, command)?)
        .status()
        .await
        .and_then(IntoResult::into_result)
        .with_context(|| format!("{} exited in failure", command))
}

async fn copy<D: AsRef<OsStr>, S: AsRef<OsStr>>(
    tmp: &Path,
    source: S,
    name: D,
) -> anyhow::Result<()> {
    let source = Path::new(source.as_ref());
    eprintln!("copying logs from {}", source.display());

    let dest = tmp.join(name.as_ref());

    if let Some(parent) = dest.parent() {
        let _ = tokio::fs::create_dir_all(&parent).await;
    }

    if source.is_file() {
        let source = async move {
            AsyncFile::open(source)
                .await
                .context("failed to open source")
        };

        let dest = async move {
            AsyncFile::create(&dest)
                .await
                .context("failed to create dest")
        };

        let (mut source, mut dest) = futures::try_join!(source, dest)?;
        tokio::io::copy(&mut source, &mut dest)
            .await
            .context("failed to copy")
            .map(|_| ())
    } else {
        if let Ok(repos) = std::fs::read_dir(source) {
            let mut tasks = Vec::new();

            for entry in repos.filter_map(Result::ok) {
                if entry.metadata().map_or(false, |m| m.is_file()) {
                    let src = entry.path();
                    if src.is_file() {
                        let name = entry.file_name().to_owned();
                        tasks.push(async {
                            let name = name;
                            let src = src;
                            copy(&src, &dest, &name).await
                        });
                    }
                }
            }

            let _ = futures::future::join_all(tasks);
        }

        Ok(())
    }
}

fn tempfile(path: &Path, command: &str) -> anyhow::Result<File> {
    File::create(path.join(command))
        .with_context(|| format!("failed to create temporary file for {}", command))
}
