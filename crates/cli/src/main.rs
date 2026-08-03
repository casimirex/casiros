//! CASIROS command-line interface.
//!
//! Provides local, serverless access to the causality graph engine and Monte
//! Carlo simulator. All commands read and write JSON.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(warnings)]
#![allow(clippy::needless_return)]

use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod commands;
mod convert;

/// CASIROS CLI — evaluate graphs, run simulations, and persist models.
#[derive(Debug, Parser)]
#[command(name = "casiros-cli", version, about)]
struct Cli {
    /// Subcommand to run.
    #[command(subcommand)]
    command: Command,
}

/// Available CLI commands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Evaluate a causality graph from a JSON request file.
    Evaluate {
        /// Path to the JSON request file.
        file: PathBuf,
    },

    /// Run a Monte Carlo simulation from a JSON request file.
    Simulate {
        /// Path to the JSON request file.
        file: PathBuf,
    },

    /// Validate a graph request and report node/edge/depth counts.
    Validate {
        /// Path to the JSON request file.
        file: PathBuf,
    },

    /// Load an engine JSON file and write a stable snapshot JSON file.
    Save {
        /// Path to the engine JSON request file.
        engine_file: PathBuf,
        /// Path where the snapshot JSON will be written.
        snapshot_file: PathBuf,
    },

    /// Load a snapshot JSON file and write an engine JSON request file.
    Load {
        /// Path to the snapshot JSON file.
        snapshot_file: PathBuf,
        /// Path where the engine JSON request will be written.
        engine_file: PathBuf,
    },

    /// Convert between JSON, CSV, and Excel file formats.
    Convert {
        /// Path to the input file (.json, .csv, or .xlsx).
        input: PathBuf,
        /// Path where the converted output will be written.
        output: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Evaluate { file } => commands::evaluate(&file),
        Command::Simulate { file } => commands::simulate(&file),
        Command::Validate { file } => commands::validate(&file),
        Command::Save {
            engine_file,
            snapshot_file,
        } => commands::save(&engine_file, &snapshot_file).map(|()| "Snapshot written".to_string()),
        Command::Load {
            snapshot_file,
            engine_file,
        } => {
            commands::load(&snapshot_file, &engine_file).map(|()| "Engine file written".to_string())
        }
        Command::Convert { input, output } => convert::convert(&input, &output),
    };

    match result {
        Ok(output) => {
            println!("{output}");
            std::process::exit(0);
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
