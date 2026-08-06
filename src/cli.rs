use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};

use crate::analysis::summary::summarize;
use crate::graph::trace_graph::TraceCollection;
use crate::input::otlp_json::parse_otlp_file;
use crate::model::diagnostic::Severity;
use crate::model::span::{TRACE_ID_LEN, normalize_hex_id};
use crate::output::text::{format_summary, format_tree, format_validate};

#[derive(Debug, Parser)]
#[command(
    name = "tracelens",
    version,
    about = "Local OpenTelemetry trace analysis CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Validate an OTLP trace file and print diagnostics.
    Validate {
        /// Path to an OTLP JSON trace file.
        file: PathBuf,

        /// Treat malformed IDs, timestamps, and required fields as fatal.
        #[arg(long)]
        strict: bool,
    },

    /// Print a file-level trace summary.
    Summary {
        /// Path to an OTLP JSON trace file.
        file: PathBuf,
    },

    /// Print a parent-child tree for a single trace.
    Tree {
        /// Path to an OTLP JSON trace file.
        file: PathBuf,

        /// Trace ID to inspect.
        #[arg(long = "trace-id")]
        trace_id: String,
    },
}

pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { file, strict } => {
            let data = parse_otlp_file(&file)
                .with_context(|| format!("failed to parse {}", file.display()))?;
            let collection = TraceCollection::build(data);
            let has_error = collection
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Severity::Error);

            print!("{}", format_validate(&file, &collection, strict));

            if strict && has_error {
                Ok(ExitCode::FAILURE)
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        Commands::Summary { file } => {
            let data = parse_otlp_file(&file)
                .with_context(|| format!("failed to parse {}", file.display()))?;
            let collection = TraceCollection::build(data);
            let summary = summarize(&collection);

            print!("{}", format_summary(&file, &summary, &collection));
            Ok(ExitCode::SUCCESS)
        }
        Commands::Tree { file, trace_id } => {
            let normalized_trace_id = normalize_hex_id(&trace_id, TRACE_ID_LEN)
                .map_err(|message| anyhow!("invalid --trace-id: {message}"))?;
            let data = parse_otlp_file(&file)
                .with_context(|| format!("failed to parse {}", file.display()))?;
            let collection = TraceCollection::build(data);
            let trace = collection
                .traces
                .get(&normalized_trace_id)
                .ok_or_else(|| anyhow!("trace_id not found: {normalized_trace_id}"))?;

            print!("{}", format_tree(trace));
            Ok(ExitCode::SUCCESS)
        }
    }
}
