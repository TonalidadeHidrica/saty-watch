use anyhow::Context;
use notify::{watcher, DebouncedEvent::*, RecursiveMode, Watcher};
use std::{path::PathBuf, process::Command, sync::mpsc, time::Duration};
use wslpath::wsl_to_windows;

use clap::Parser;

fn is_satysfi_related(path: PathBuf) -> bool {
    match path.extension().and_then(|a| a.to_str()) {
        Some("saty" | "satyg") => true,
        Some(ex) => ex.starts_with("satyh"),
        None => false,
    }
}

#[derive(Parser)]
struct Opts {
    target_file: PathBuf,
    watch_dirs: Vec<PathBuf>,
    #[clap(short, long)]
    output: Option<PathBuf>,
    #[clap(long)]
    extra_args: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();

    let target_file = opts.target_file;
    let output_path = opts.output;

    let (tx, rx) = mpsc::channel();
    let mut watcher = watcher(tx, Duration::from_millis(500))?;

    let target_parent = target_file
        .parent()
        .expect("Not a file but a directory was specified");
    if target_parent.as_os_str().is_empty() {
        watcher.watch(".", RecursiveMode::Recursive)?;
    } else {
        watcher.watch(target_parent, RecursiveMode::Recursive)?;
    }

    for watch_dir in opts.watch_dirs {
        watcher.watch(watch_dir, RecursiveMode::Recursive)?;
    }

    let mut skim_open = false;

    let mut command = Command::new("satysfi");
    command.arg(&target_file);
    if let Some(path) = &output_path {
        command.arg("-o").arg(path);
    }
    if let Some(extra_args) = &opts.extra_args {
        for arg in extra_args.split_whitespace() {
            command.arg(arg);
        }
    }
    for event in rx.iter() {
        let condition = match event {
            Create(path) | Write(path) | Chmod(path) | Remove(path) => is_satysfi_related(path),
            Rename(path1, path2) => is_satysfi_related(path1) || is_satysfi_related(path2),
            _ => false,
        };
        if condition {
            let status = command.status()?;
            if status.success() && !skim_open {
                skim_open = true;
                let with_extension = target_file.with_extension("pdf");
                let output_path = output_path.as_ref().unwrap_or(&with_extension);
                if wsl::is_wsl() {
                    let output_path = wsl_to_windows(
                        output_path
                            .to_str()
                            .context("Could not convert output path to string")?,
                    )
                    .expect("Failed to convert wsl path to windows path")
                    .replace('/', "\\");  // <= dirty hack, should be handled by wslpath crate
                    Command::new("explorer.exe")
                        .arg(dbg!(output_path))
                        .output()?;
                } else {
                    Command::new("open")
                        .args(["-a", "Skim"])
                        .arg(output_path)
                        .output()?;
                }
            }
        }
    }
    Ok(())
}
