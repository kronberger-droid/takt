use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};

use crate::{
    error::TaktError,
    report::{Period, Report, ReportRange},
    store::{FlatStore, Store},
};

mod error;
mod log;
mod model;
mod report;
mod store;
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
    let mut store: Box<dyn Store> = Box::new(FlatStore::new(data_dir()?));

    match cli.command {
        Commands::Start { tag } => {
            let resolved_tag = store.tag_resolve(&tag)?;
            store.start(&resolved_tag)?;
        }
        Commands::Stop => {
            store.stop()?;
        }
        Commands::Status => {
            let log = store.active()?;
            if let Some(active) = log {
                let tag = &active.tag;
                let elapsed = chrono::Local::now().naive_local() - active.start;

                let hours = elapsed.num_hours();
                let minutes = elapsed.num_minutes() % 60;

                println!("Tracking: {tag}");
                println!("Elapsed: {hours}h {minutes}m")
            } else {
                println!("Not tracking anything.");
            }
        }
        Commands::Report { range } => {
            let range = range.unwrap_or(ReportRange::This {
                period: Period::Week,
            });
            let (start, end) = range.date_range();
            let entries = store.entries_between(start, end)?;
            let report = Report::generate(&entries, range);
            print!("{}", report.display());
        }
        Commands::Tag { action } => match action {
            TagCommands::Add { path } => {
                store.tag_add(&path)?;
                println!("added tag: {path}");
            }
            TagCommands::List => {
                print! {"{}", store.tag_list()?};
            }
        },
    }

    Ok(())
}

fn data_dir() -> Result<PathBuf, TaktError> {
    Ok(dirs::data_dir().ok_or(TaktError::NoDataDir)?.join("takt"))
}
