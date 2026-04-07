use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{error::TaktError, log::TaskLog, tags::TagTree};

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
}

#[derive(Subcommand, Clone)]
enum TagCommands {
    /// Add a new tag path (e.g. work/project-x/task)
    Add { path: String },
    /// Show the tag tree
    List,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let tags_path = dirs::data_dir().unwrap().join("takt/tags");

    match cli.command {
        Commands::Start { tag } => {
            let tree = TagTree::load(&tags_path)?;
            let resolved = tree.resolve(&tag)?;
            let log_path = log_path_for_month(0);
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
            let active = log.active().expect("Could not find an active Entry");

            let tag = &active.tag;
            let elapsed = chrono::Local::now().naive_local() - active.start;

            let hours = elapsed.num_hours();
            let minutes = elapsed.num_minutes() % 60;

            println!("Tracking: {tag}");
            println!("Elapsed: {hours}h {minutes}m")
        }
        Commands::Tag { action } => match action {
            TagCommands::Add { path } => {
                let mut tree = TagTree::load(&tags_path)?;
                tree.add(&path);
                tree.save(&tags_path)?;
            }
            TagCommands::List => {
                let tree = TagTree::load(&tags_path)?;
                print! {"{}", tree.write()};
            }
        },
    }

    Ok(())
}

fn find_active_log() -> Result<(TaskLog, PathBuf), TaktError> {
    let path = log_path_for_month(0);
    let log = TaskLog::load(&path)?;
    if log.active().is_some() {
        return Ok((log, path));
    }

    let prev_path = log_path_for_month(1);
    let prev_log = TaskLog::load(&prev_path)?;
    if prev_log.active().is_some() {
        return Ok((prev_log, prev_path));
    }

    Err(TaktError::NoActiveTask)
}

fn log_path_for_month(months_ago: u32) -> PathBuf {
    let base = dirs::data_dir().unwrap().join("takt");
    let now =
        chrono::Local::now().date_naive() - chrono::Months::new(months_ago);
    base.join(format!("log/{}.takt", now.format("%Y-%m")))
}
