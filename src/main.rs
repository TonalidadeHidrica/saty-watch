use notify::{watcher, DebouncedEvent::*, RecursiveMode, Watcher};
use std::{path::PathBuf, process::Command, sync::mpsc, time::Duration};

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
    extra_args: String,
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
    for arg in opts.extra_args.trim().split_whitespace() {
        command.arg(arg);
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
                Command::new("open")
                    .args(["-a", "Skim"])
                    .arg(output_path)
                    .output()?;
            }
        }
    }
    Ok(())
}
