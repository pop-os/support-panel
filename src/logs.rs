// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use anyhow::Context;
use as_result::IntoResult;
use smol::fs::File as AsyncFile;
use smol::process::Command;
use std::ffi::OsStr;
use std::{fs::File, path::Path, process::Stdio};

pub async fn generate(home: &str) -> anyhow::Result<String> {
    let tempdir = tempfile::tempdir().context("failed to fetch temporary directory")?;

    async fn system_info(file: File) -> anyhow::Result<()> {
        use futures::io::AsyncWriteExt;

        let info = crate::support_info::SupportInfo::fetch().await;

        let data = fomat_macros::fomat! {
            "Model: " (info.model_and_version) "\n"
            "OS Version: " (info.operating_system) "\n"
            "Kernel Version: " (info.kernel_version) "\n"
            "Kernel Revision: " (info.kernel_revision) "\n"
        };

        let mut file = AsyncFile::from(file);

        file.write_all(data.as_bytes())
            .await
            .context("failed to write system info")?;

        dbg!(file.flush().await.context("failed to write system info"))
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
    async fn copy_<D: AsRef<OsStr>, S: AsRef<OsStr>>(
        tmp: &Path,
        source: S,
        name: D,
    ) -> anyhow::Result<()> {
        let source = Path::new(source.as_ref());
        let dest = tmp.join(name.as_ref());

        eprintln!(
            "copying logs from {} to {}",
            source.display(),
            dest.display()
        );

        if let Some(parent) = dest.parent() {
            let _ = smol::fs::create_dir_all(&parent).await;
        }

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
        smol::io::copy(&mut source, &mut dest)
            .await
            .context("failed to copy")
            .map(|_| ())
    }

    let source = Path::new(source.as_ref());

    if source.is_file() {
        copy_(tmp, source, name).await
    } else {
        let dest = tmp.join(name.as_ref());

        if let Ok(repos) = std::fs::read_dir(source) {
            let mut tasks = Vec::new();

            for entry in repos.filter_map(Result::ok) {
                if entry.metadata().map_or(false, |m| m.is_file()) {
                    let src = entry.path();
                    let dest = dest.join(entry.file_name());
                    if src.is_file() {
                        tasks.push(async move {
                            let src = src;
                            copy_(tmp, &src, &dest).await
                        });
                    }
                }
            }

            let _ = futures::future::join_all(tasks).await;
        }

        Ok(())
    }
}

fn tempfile(path: &Path, command: &str) -> anyhow::Result<File> {
    File::create(path.join(command))
        .with_context(|| format!("failed to create temporary file for {}", command))
}
