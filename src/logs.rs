// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use anyhow::Context;
use as_result::IntoResult;
use async_pidfd::AsyncPidFd;
use compio_fs::File;
use compio_io::{AsyncReadAt, AsyncWriteAt, AsyncWriteAtExt};
use std::ffi::OsStr;
use std::io;
use std::process::Command;
use std::{path::Path, process::Stdio};

pub async fn generate(home: &str) -> anyhow::Result<String> {
    let tempdir = tempfile::tempdir().context("failed to fetch temporary directory")?;

    async fn system_info(path: &Path, command: &str) -> anyhow::Result<()> {
        let mut file = File::create(path.join(command))
            .await
            .context("failed to create file for system_info")?;

        let info = crate::support_info::SupportInfo::fetch().await;

        let data = fomat_macros::fomat! {
            "Model: " (info.model_and_version) "\n"
            "OS Version: " (info.operating_system) "\n"
            "Kernel Version: " (info.kernel_version) "\n"
            "Kernel Revision: " (info.kernel_revision) "\n"
        };

        file.write_all_at(data, 0)
            .await
            .0
            .context("failed to write system info")
    }

    let temp = tempdir.path();

    let _ = futures::join!(
        command("libinput", &["list-devices"], temp, "libinput"),
        command("cosmic-randr", &["list", "--kdl"], temp, "cosmic-randr"),
        command("df", &["-h"], temp, "free-disk-space"),
        command("dmesg", &[], temp, "dmesg"),
        command("dmidecode", &[], temp, "dmidecode"),
        command("efibootmgr", &["-v"], temp, "efibootmgr"),
        command("journalctl", &["--since", "yesterday"], temp, "journalctl"),
        command(
            "lsblk",
            &[
                "-o",
                "NAME,MODEL,FSTYPE,FSVER,SIZE,FSUSE%,MOUNTPOINTS,LABEL,UUID"
            ],
            temp,
            "lsblk"
        ),
        command("last", &[], temp, "reboot-history"),
        command("lspci", &["-vv"], temp, "lspci"),
        command("lsusb", &["-vv"], temp, "lsusb"),
        command("lsmod", &[], temp, "lsmod"),
        command("sensors", &[], temp, "sensors"),
        command("systemd-analyze", &["blame"], temp, "boot-process-times"),
        command("upower", &["-d"], temp, "upower"),
        command("uptime", &[], temp, "uptime"),
        copy(temp, "/etc/apt/sources.list.d", "apt/sources.list.d"),
        copy(temp, "/etc/apt/sources.list", "apt/sources.list"),
        copy(temp, "/etc/crypttab", "crypttab"),
        copy(temp, "/etc/fstab", "fstab"),
        copy(temp, "/etc/kernelstub/configuration", "kernelstub"),
        copy(temp, "/var/log/apt/history.log", "apt/history.log"),
        copy(
            temp,
            "/var/log/apt/history.log.1.gz",
            "apt/history-rotated.log.gz"
        ),
        copy(temp, "/var/log/apt/term.log", "apt/term.log"),
        copy(
            temp,
            "/var/log/apt/term.log.1.gz",
            "apt/term-rotated.log.gz"
        ),
        copy(temp, "/var/log/syslog", "syslog.log"),
        copy(temp, "/var/log/Xorg.0.log", "Xorg.0.log"),
        system_info(temp, "systeminfo.txt")
    );

    let files_to_collect: Vec<String> = std::fs::read_dir(temp)
        .map(|dir| {
            dir.filter_map(Result::ok)
                .filter_map(|entry| Some(entry.file_name().to_str()?.to_owned()))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    eprintln!("logs generated {:?}", files_to_collect);

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
        .spawn()
        .and_then(|child| AsyncPidFd::from_pid(child.id() as i32))
        .unwrap()
        .wait()
        .await
        .and_then(|info| info.status().into_result())
        .context("tar exited in failure")?;

    Ok(log_path)
}

async fn command(command: &str, args: &[&str], temp: &Path, filename: &str) -> anyhow::Result<()> {
    eprintln!("run `{command}`");
    Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(tempfile(temp, filename)?)
        .spawn()
        .and_then(|child| AsyncPidFd::from_pid(child.id() as i32))
        .unwrap()
        .wait()
        .await
        .and_then(|info| info.status().into_result())
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
        let source = Path::new(source.as_ref()).to_path_buf();
        let dest = tmp.join(name.as_ref());

        eprintln!("copying {}", source.display());

        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(&parent);
        }

        let mut source = File::open(source)
            .await
            .context("failed to open file for copying")?;

        let mut dest = File::create(dest)
            .await
            .context("failed to open file for copying")?;

        fs_copy(&mut source, &mut dest)
            .await
            .context("failed to copy file")?;

        Ok(())
    }

    let source = Path::new(source.as_ref());

    let res = if source.is_file() {
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
    };

    eprintln!("COPY DONE");

    res
}

fn tempfile(path: &Path, command: &str) -> anyhow::Result<std::fs::File> {
    std::fs::File::create(path.join(command))
        .with_context(|| format!("failed to create temporary file for {}", command))
}

async fn fs_copy(reader: &mut File, writer: &mut File) -> io::Result<u64> {
    let mut buf = Vec::with_capacity(8 * 1024);
    let mut total = 0u64;
    let mut pos = 0;

    loop {
        let res;
        (res, buf) = reader.read_at(buf, pos).await.into();
        match res {
            Ok(0) => break,
            Ok(read) => {
                total += read as u64;
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                continue;
            }
            Err(e) => return Err(e),
        }
        let res;
        (res, buf) = writer.write_at(buf, pos).await.into();
        res?;
        pos = total;
        buf.clear();
    }

    Ok(total)
}
