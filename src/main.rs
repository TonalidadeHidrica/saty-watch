use notify::{watcher, DebouncedEvent::*, RecursiveMode, Watcher};
use std::{path::PathBuf, process::Command, sync::mpsc, time::Duration};

use clap::{App, Arg};

fn is_satysfi_related(path: PathBuf) -> bool {
    match path.extension().and_then(|a| a.to_str()) {
        Some("saty" | "satyg") => true,
        Some(ex) => ex.starts_with("satyh"),
        None => false,
    }
}

fn main() -> anyhow::Result<()> {
    let matches = App::new("saty-watch")
        .arg(Arg::with_name("target_file").required(true))
        .arg(Arg::with_name("watch_dirs").multiple(true))
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true),
        )
        .get_matches();
    let target_file = PathBuf::from(matches.value_of("target_file").unwrap());
    let target_parent = target_file
        .parent()
        .expect("Not a file but a directory was specified");
    let output_path = matches.value_of("output").map(PathBuf::from);

    let (tx, rx) = mpsc::channel();
    let mut watcher = watcher(tx, Duration::from_millis(500))?;

    watcher.watch(target_parent, RecursiveMode::Recursive)?;

    for watch_dir in matches.values_of("watch_dirs").iter_mut().flatten() {
        watcher.watch(watch_dir, RecursiveMode::Recursive)?;
    }

    let mut skim_open = false;

    let mut command = Command::new("satysfi");
    command.arg(&target_file);
    if let Some(path) = &output_path {
        command.arg("-o").arg(path);
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
