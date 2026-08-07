use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand, ValueEnum};

use crate::analysis::annotations::annotate_trace_spans;
use crate::analysis::classification::classify_trace_spans;
use crate::analysis::critical_path::analyze_critical_path;
use crate::analysis::duration::analyze_trace_duration;
use crate::analysis::summary::{TraceSummary, summarize};
use crate::graph::trace_graph::TraceCollection;
use crate::input::otlp_json::parse_otlp_file;
use crate::model::diagnostic::Severity;
use crate::model::span::{TRACE_ID_LEN, normalize_hex_id};
use crate::output::json::{
    format_critical_path_json, format_list_traces_json, format_services_json, format_summary_json,
    format_tree_json, format_validate_json,
};
use crate::output::style::{ColorMode, TextStyle};
use crate::output::text::{
    format_critical_path, format_list_traces, format_services, format_summary, format_tree,
    format_validate,
};

#[derive(Debug, Parser)]
#[command(
    name = "tracelens",
    version,
    about = "Local OpenTelemetry trace analysis CLI"
)]
struct Cli {
    /// Colorize text output.
    #[arg(long, value_enum, global = true, default_value_t = ColorChoice::Auto)]
    color: ColorChoice,

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

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        output: OutputFormat,
    },

    /// Print a file-level trace summary.
    Summary {
        /// Path to an OTLP JSON trace file.
        file: PathBuf,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        output: OutputFormat,
    },

    /// List traces sorted by a simple metric.
    ListTraces {
        /// Path to an OTLP JSON trace file.
        file: PathBuf,

        /// Maximum number of traces to print.
        #[arg(long, default_value_t = 20)]
        limit: usize,

        /// Sort metric.
        #[arg(long, value_enum, default_value_t = TraceSort::Duration)]
        sort: TraceSort,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        output: OutputFormat,
    },

    /// Print a parent-child tree for a single trace.
    Tree {
        /// Path to an OTLP JSON trace file.
        file: PathBuf,

        /// Trace ID to inspect.
        #[arg(long = "trace-id")]
        trace_id: String,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        output: OutputFormat,
    },

    /// Print the critical path and span execution classification for a single trace.
    CriticalPath {
        /// Path to an OTLP JSON trace file.
        file: PathBuf,

        /// Trace ID to inspect.
        #[arg(long = "trace-id")]
        trace_id: String,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        output: OutputFormat,
    },

    /// Explain service-level duration and self time for a single trace.
    Services {
        /// Path to an OTLP JSON trace file.
        file: PathBuf,

        /// Trace ID to inspect.
        #[arg(long = "trace-id")]
        trace_id: String,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        output: OutputFormat,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}

impl From<ColorChoice> for ColorMode {
    fn from(value: ColorChoice) -> Self {
        match value {
            ColorChoice::Auto => Self::Auto,
            ColorChoice::Always => Self::Always,
            ColorChoice::Never => Self::Never,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum TraceSort {
    Duration,
    Spans,
    Errors,
}

pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let text_style = TextStyle::from_mode(cli.color.into());

    match cli.command {
        Commands::Validate {
            file,
            strict,
            output,
        } => {
            let collection = load_collection(&file)?;
            let has_error = collection
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Severity::Error);

            match output {
                OutputFormat::Text => {
                    print!(
                        "{}",
                        format_validate(&file, &collection, strict, text_style)
                    )
                }
                OutputFormat::Json => print!("{}", format_validate_json(&collection, strict)),
            }

            if strict && has_error {
                Ok(ExitCode::FAILURE)
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        Commands::Summary { file, output } => {
            let collection = load_collection(&file)?;
            ensure_has_spans(&collection)?;
            let summary = summarize(&collection);

            match output {
                OutputFormat::Text => {
                    print!(
                        "{}",
                        format_summary(&file, &summary, &collection, text_style)
                    )
                }
                OutputFormat::Json => print!("{}", format_summary_json(&summary, &collection)),
            }
            Ok(ExitCode::SUCCESS)
        }
        Commands::ListTraces {
            file,
            limit,
            sort,
            output,
        } => {
            if limit == 0 {
                return Err(anyhow!("--limit must be greater than 0"));
            }

            let collection = load_collection(&file)?;
            ensure_has_spans(&collection)?;
            let traces = sorted_trace_summaries(&collection, sort);

            match output {
                OutputFormat::Text => {
                    print!("{}", format_list_traces(&file, &traces, limit, text_style))
                }
                OutputFormat::Json => print!("{}", format_list_traces_json(&traces, limit)),
            }
            Ok(ExitCode::SUCCESS)
        }
        Commands::Tree {
            file,
            trace_id,
            output,
        } => {
            let normalized_trace_id = normalize_hex_id(&trace_id, TRACE_ID_LEN)
                .map_err(|message| anyhow!("invalid --trace-id: {message}"))?;
            let collection = load_collection(&file)?;
            ensure_has_spans(&collection)?;
            let trace = collection
                .traces
                .get(&normalized_trace_id)
                .ok_or_else(|| anyhow!("trace_id not found: {normalized_trace_id}"))?;

            let annotations = annotate_trace_spans(trace);

            match output {
                OutputFormat::Text => print!("{}", format_tree(trace, &annotations, text_style)),
                OutputFormat::Json => print!("{}", format_tree_json(trace, &annotations)),
            }
            Ok(ExitCode::SUCCESS)
        }
        Commands::CriticalPath {
            file,
            trace_id,
            output,
        } => {
            let normalized_trace_id = normalize_hex_id(&trace_id, TRACE_ID_LEN)
                .map_err(|message| anyhow!("invalid --trace-id: {message}"))?;
            let collection = load_collection(&file)?;
            ensure_has_spans(&collection)?;
            let trace = collection
                .traces
                .get(&normalized_trace_id)
                .ok_or_else(|| anyhow!("trace_id not found: {normalized_trace_id}"))?;
            let duration = analyze_trace_duration(trace);
            let critical_path = analyze_critical_path(trace);
            let classification = classify_trace_spans(trace);
            let annotations = annotate_trace_spans(trace);

            match output {
                OutputFormat::Text => print!(
                    "{}",
                    format_critical_path(
                        &duration,
                        &critical_path,
                        &classification,
                        &annotations,
                        trace,
                        text_style
                    )
                ),
                OutputFormat::Json => print!(
                    "{}",
                    format_critical_path_json(
                        &duration,
                        &critical_path,
                        &classification,
                        &annotations,
                        trace
                    )
                ),
            }
            Ok(ExitCode::SUCCESS)
        }
        Commands::Services {
            file,
            trace_id,
            output,
        } => {
            let normalized_trace_id = normalize_hex_id(&trace_id, TRACE_ID_LEN)
                .map_err(|message| anyhow!("invalid --trace-id: {message}"))?;
            let collection = load_collection(&file)?;
            ensure_has_spans(&collection)?;
            let trace = collection
                .traces
                .get(&normalized_trace_id)
                .ok_or_else(|| anyhow!("trace_id not found: {normalized_trace_id}"))?;
            let analysis = analyze_trace_duration(trace);

            match output {
                OutputFormat::Text => print!(
                    "{}",
                    format_services(&analysis, &trace.diagnostics, text_style)
                ),
                OutputFormat::Json => print!("{}", format_services_json(&analysis, trace)),
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn load_collection(file: &Path) -> Result<TraceCollection> {
    let data =
        parse_otlp_file(file).with_context(|| format!("failed to parse {}", file.display()))?;
    Ok(TraceCollection::build(data))
}

fn ensure_has_spans(collection: &TraceCollection) -> Result<()> {
    if collection.span_count() == 0 {
        return Err(anyhow!(
            "no valid spans found; run validate to inspect diagnostics"
        ));
    }

    Ok(())
}

fn sorted_trace_summaries(collection: &TraceCollection, sort: TraceSort) -> Vec<TraceSummary> {
    let mut traces = summarize(collection).slowest_traces;
    match sort {
        TraceSort::Duration => traces.sort_by(|left, right| {
            right
                .duration_ns
                .cmp(&left.duration_ns)
                .then(left.trace_id.cmp(&right.trace_id))
        }),
        TraceSort::Spans => traces.sort_by(|left, right| {
            right
                .span_count
                .cmp(&left.span_count)
                .then(left.trace_id.cmp(&right.trace_id))
        }),
        TraceSort::Errors => traces.sort_by(|left, right| {
            right
                .error_span_count
                .cmp(&left.error_span_count)
                .then(left.trace_id.cmp(&right.trace_id))
        }),
    }
    traces
}
