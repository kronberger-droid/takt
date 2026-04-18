use std::{path::PathBuf, process::ExitCode};

use chrono::Datelike;
use clap::{Parser, Subcommand};

use crate::{
    error::TaktError,
    log::TaskLog,
    report::{Period, Report, ReportRange},
    tags::TagTree,
};

mod error;
mod log;
mod report;
mod tags;

#[derive(Parser)]
#[command(name = "takt", about = "Time tracking with focus")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    /// Start tracking a task
    Start { tag: String },
    /// Stop the current task
    Stop,
    /// Shows what's currently being tracked
    Status,
    /// Manage Tags
    Tag {
        #[command(subcommand)]
        action: TagCommands,
    },
    /// Summarize tracked time for a range (defaults to `this week`)
    Report {
        #[command(subcommand)]
        range: Option<ReportRange>,
    },
}

#[derive(Subcommand, Clone)]
enum TagCommands {
    /// Add a new tag path (e.g. work/project-x/task)
    Add { path: String },
    /// Show the tag tree
    List,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), TaktError> {
    let cli = Cli::parse();
    let tags_path = data_dir()?.join("tags");

    match cli.command {
        Commands::Start { tag } => {
            let tree = TagTree::load(&tags_path)?;
            let resolved = tree.resolve(&tag)?;
            let log_path = log_path_for_month(0)?;
            let mut log = TaskLog::load(&log_path)?;
            log.start(&resolved)?;
            log.save(&log_path)?;
        }
        Commands::Stop => {
            let (mut active, path) = find_active_log()?;
            active.stop()?;
            active.save(&path)?;
        }
        Commands::Status => {
            let (log, _) = find_active_log()?;
            let active = log.active().ok_or(TaktError::NoActiveTask)?;

            let tag = &active.tag;
            let elapsed = chrono::Local::now().naive_local() - active.start;

            let hours = elapsed.num_hours();
            let minutes = elapsed.num_minutes() % 60;

            println!("Tracking: {tag}");
            println!("Elapsed: {hours}h {minutes}m")
        }
        Commands::Report { range } => {
            let range = range.unwrap_or(ReportRange::This {
                period: Period::Week,
            });
            let entries = gather_entries_for_range(&range)?;
            let report = Report::generate(&entries, range);
            print!("{}", report.display());
        }
        Commands::Tag { action } => match action {
            TagCommands::Add { path } => {
                let mut tree = TagTree::load(&tags_path)?;
                tree.add(&path);
                tree.save(&tags_path)?;
                println!("added tag: {path}");
            }
            TagCommands::List => {
                let tree = TagTree::load(&tags_path)?;
                print! {"{}", tree.write()};
            }
        },
    }

    Ok(())
}

/// Collect all log entries that could fall inside `range`.
/// Missing monthly files are silently skipped — only malformed ones warn.
fn gather_entries_for_range(
    range: &ReportRange,
) -> Result<Vec<log::Entry>, TaktError> {
    let (start, _) = range.date_range();
    let today = chrono::Local::now().date_naive();

    let mut cursor = start.date().with_day(1).unwrap();
    let end_month = today.with_day(1).unwrap();
    let mut entries = Vec::new();

    while cursor <= end_month {
        let path = log_path_for_date(cursor)?;
        if path.exists() {
            match TaskLog::load(&path) {
                Ok(log) => entries.extend(log.entries().iter().cloned()),
                Err(e) => eprintln!(
                    "warning: failed to load {} ({e}) — skipped",
                    cursor.format("%Y-%m")
                ),
            }
        }
        cursor = cursor + chrono::Months::new(1);
    }
    Ok(entries)
}

fn find_active_log() -> Result<(TaskLog, PathBuf), TaktError> {
    let path = log_path_for_month(0)?;
    let log = TaskLog::load(&path)?;
    if log.active().is_some() {
        return Ok((log, path));
    }

    let prev_path = log_path_for_month(1)?;
    let prev_log = TaskLog::load(&prev_path)?;
    if prev_log.active().is_some() {
        return Ok((prev_log, prev_path));
    }

    Err(TaktError::NoActiveTask)
}

fn data_dir() -> Result<PathBuf, TaktError> {
    Ok(dirs::data_dir().ok_or(TaktError::NoDataDir)?.join("takt"))
}

fn log_path_for_month(months_ago: u32) -> Result<PathBuf, TaktError> {
    let date =
        chrono::Local::now().date_naive() - chrono::Months::new(months_ago);
    log_path_for_date(date)
}

fn log_path_for_date(date: chrono::NaiveDate) -> Result<PathBuf, TaktError> {
    Ok(data_dir()?.join(format!("log/{}.takt", date.format("%Y-%m"))))
}
