use getclean::folders::Folders;
use getclean::history::History;
use getclean::Organizer;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

struct Options {
    folders: Folders,
    interval: Duration,
    settle: Duration,
    once: bool,
    dry_run: bool,
    show_history: bool,
    undo: bool,
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(Some(options)) => options,
        Ok(None) => return ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("getClean: {message}\nRun getclean --help for usage.");
            return ExitCode::FAILURE;
        }
    };

    let history = History::new(options.folders.state.join("history.log"));
    if options.show_history {
        match history.entries() {
            Ok(entries) => {
                for entry in entries.iter().rev().take(20) {
                    let status = if entry.restored.is_some() {
                        "undone"
                    } else {
                        "moved"
                    };
                    println!(
                        "{} [{status}] {} -> {}",
                        entry.timestamp,
                        entry.source.display(),
                        entry.destination.display()
                    );
                }
                return ExitCode::SUCCESS;
            }
            Err(err) => {
                eprintln!("getClean: cannot read history: {err}");
                return ExitCode::FAILURE;
            }
        }
    }
    if options.undo {
        match history.undo_latest() {
            Ok(Some((from, to))) => println!("restored {} -> {}", from.display(), to.display()),
            Ok(None) => println!("nothing to undo"),
            Err(err) => {
                eprintln!("getClean: undo failed: {err}");
                return ExitCode::FAILURE;
            }
        }
        return ExitCode::SUCCESS;
    }

    if !options.folders.downloads.is_dir() {
        eprintln!(
            "getClean: Downloads directory does not exist: {}",
            options.folders.downloads.display()
        );
        return ExitCode::FAILURE;
    }
    let mut organizer = Organizer::new(options.folders, options.settle);
    if options.once || options.dry_run {
        match organizer.scan(options.dry_run) {
            Ok(result) => {
                println!(
                    "moved: {}, waiting: {}, failed: {}",
                    result.moved, result.waiting, result.failed
                );
                return if result.failed == 0 {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                };
            }
            Err(err) => {
                eprintln!("getClean: {err}");
                return ExitCode::FAILURE;
            }
        }
    }

    println!(
        "getClean watching {} every {} seconds",
        organizer.downloads().display(),
        options.interval.as_secs()
    );
    loop {
        if let Err(err) = organizer.scan(false) {
            // Keep running through transient I/O errors such as a disconnected drive.
            eprintln!("getClean: scan failed: {err}");
        }
        thread::sleep(options.interval);
    }
}

fn parse_args() -> Result<Option<Options>, String> {
    if env::args_os()
        .skip(1)
        .any(|arg| matches!(arg.to_str(), Some("--help" | "-h")))
    {
        print_help();
        return Ok(None);
    }
    let mut folders =
        Folders::discover().map_err(|err| format!("cannot locate user folders: {err}"))?;
    if let Some(downloads) = env::var_os("GETCLEAN_DOWNLOADS") {
        folders.downloads = PathBuf::from(downloads);
    }
    let mut interval = Duration::from_secs(5);
    let mut settle = Duration::from_secs(10);
    let mut once = false;
    let mut dry_run = false;
    let mut show_history = false;
    let mut undo = false;
    let mut args = env::args_os().skip(1);

    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--help" | "-h") => {
                print_help();
                return Ok(None);
            }
            Some("--downloads") => {
                folders.downloads = PathBuf::from(args.next().ok_or("--downloads needs a path")?)
            }
            Some("--state-dir") => {
                folders.state = PathBuf::from(args.next().ok_or("--state-dir needs a path")?)
            }
            Some("--interval") => interval = seconds(args.next(), "--interval")?,
            Some("--settle") => settle = seconds(args.next(), "--settle")?,
            Some("--once") => once = true,
            Some("--dry-run") => dry_run = true,
            Some("--history") => show_history = true,
            Some("--undo") => undo = true,
            _ => return Err(format!("unknown argument: {}", arg.to_string_lossy())),
        }
    }
    if show_history && undo {
        return Err("--history and --undo cannot be combined".to_string());
    }
    let current_dir =
        env::current_dir().map_err(|err| format!("cannot read current directory: {err}"))?;
    if folders.downloads.is_relative() {
        folders.downloads = current_dir.join(folders.downloads);
    }
    if folders.state.is_relative() {
        folders.state = current_dir.join(folders.state);
    }
    Ok(Some(Options {
        folders,
        interval,
        settle,
        once,
        dry_run,
        show_history,
        undo,
    }))
}

fn seconds(value: Option<std::ffi::OsString>, flag: &str) -> Result<Duration, String> {
    let value = value.ok_or_else(|| format!("{flag} needs a number of seconds"))?;
    let seconds: u64 = value
        .to_str()
        .ok_or_else(|| format!("{flag} must be a number"))?
        .parse()
        .map_err(|_| format!("{flag} must be a positive whole number"))?;
    if seconds == 0 || seconds > 3600 {
        return Err(format!("{flag} must be between 1 and 3600 seconds"));
    }
    Ok(Duration::from_secs(seconds))
}

fn print_help() {
    println!("getClean - organize files directly inside Downloads\n\
Usage: getclean [--downloads PATH] [--state-dir PATH] [--interval SECONDS] [--settle SECONDS] [--once] [--dry-run] [--history] [--undo]\n\
\n\
  --downloads PATH    Folder to organize (default: system Downloads or GETCLEAN_DOWNLOADS)\n\
  --state-dir PATH    Where to keep history (default: user local state)\n\
  --interval SECONDS  Time between scans (default: 5)\n\
  --settle SECONDS    How long a new file must remain unchanged (default: 10)\n\
  --once              Scan once and exit; recent files may still be waiting\n\
  --dry-run           Show ready files without moving them; implies --once\n\
  --history           Show the latest 20 moves\n\
  --undo              Restore the most recent move\n");
}
