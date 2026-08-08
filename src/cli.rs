use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand, ValueEnum};

use crate::analysis::annotations::annotate_trace_spans;
use crate::analysis::classification::classify_trace_spans;
use crate::analysis::critical_path::analyze_critical_path;
use crate::analysis::detect::analyze_detect;
use crate::analysis::duration::analyze_trace_duration;
use crate::analysis::summary::{TraceSummary, summarize};
use crate::analysis::timeline::{
    DEFAULT_TIMELINE_MAX_ROWS, DEFAULT_TIMELINE_WIDTH, MAX_TIMELINE_WIDTH, MIN_TIMELINE_WIDTH,
    analyze_timeline,
};
use crate::exit_code;
use crate::graph::trace_graph::TraceCollection;
use crate::input::otlp_json::parse_otlp_file;
use crate::model::diagnostic::Severity;
use crate::model::span::{TRACE_ID_LEN, normalize_hex_id};
use crate::output::html::render_html_report;
use crate::output::json::{
    format_critical_path_json, format_detect_json, format_list_traces_json, format_services_json,
    format_summary_json, format_timeline_json, format_tree_json, format_validate_json,
};
use crate::output::schema::{SchemaCommand, format_schema_json, format_schema_text};
use crate::output::style::{ColorMode, TextStyle};
use crate::output::text::{
    format_critical_path, format_detect, format_list_traces, format_services, format_summary,
    format_timeline, format_tree, format_validate,
};

#[derive(Debug, Parser)]
#[command(
    name = "tracelens",
    version,
    about = "Local OpenTelemetry trace analysis CLI",
    long_about = "Local OpenTelemetry trace analysis CLI.\n\nOutput schema:\n  Run `tracelens schema --output json` for the full JSON Schema.\n  Run `tracelens schema --output text` for field descriptions."
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
    #[command(
        after_help = "For JSON field descriptions, run `tracelens schema --command validate --output text`."
    )]
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
    #[command(
        after_help = "For JSON field descriptions, run `tracelens schema --command summary --output text`."
    )]
    Summary {
        /// Path to an OTLP JSON trace file.
        file: PathBuf,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        output: OutputFormat,
    },

    /// List traces sorted by a simple metric.
    #[command(
        after_help = "For JSON field descriptions, run `tracelens schema --command list-traces --output text`."
    )]
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

    /// Detect common performance and error candidates across traces.
    #[command(
        after_help = "For JSON field descriptions, run `tracelens schema --command detect --output text`."
    )]
    Detect {
        /// Path to an OTLP JSON trace file.
        file: PathBuf,

        /// Maximum number of candidates to print per category.
        #[arg(long, default_value_t = 5)]
        limit: usize,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        output: OutputFormat,
    },

    /// Print a parent-child tree for a single trace.
    #[command(
        after_help = "For JSON field descriptions, run `tracelens schema --command tree --output text`."
    )]
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
    #[command(
        after_help = "For JSON field descriptions, run `tracelens schema --command critical-path --output text`."
    )]
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

    /// Print an ASCII timeline for a single trace.
    #[command(
        after_help = "For JSON field descriptions, run `tracelens schema --command timeline --output text`."
    )]
    Timeline {
        /// Path to an OTLP JSON trace file.
        file: PathBuf,

        /// Trace ID to inspect.
        #[arg(long = "trace-id")]
        trace_id: String,

        /// Width of the ASCII time bar, not the whole terminal line.
        #[arg(long, default_value_t = DEFAULT_TIMELINE_WIDTH)]
        width: usize,

        /// Timeline layout mode: `bar` for the horizontal time axis, `flame`
        /// for a vertically indented ASCII flame graph.
        #[arg(long, value_enum, default_value_t = TimelineMode::Bar)]
        mode: TimelineMode,

        /// Maximum number of timeline rows before non-essential middle rows are
        /// collapsed into summary marker rows. `0` disables collapse.
        #[arg(long, default_value_t = DEFAULT_TIMELINE_MAX_ROWS)]
        max_rows: usize,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        output: OutputFormat,
    },

    /// Explain service-level duration and self time for a single trace.
    #[command(
        after_help = "For JSON field descriptions, run `tracelens schema --command services --output text`."
    )]
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

    /// Generate a single-page offline HTML report for a trace.
    #[command(
        after_help = "Report output is an HTML file (not stdout JSON); it reuses the services / critical-path / tree analysis and is not part of `tracelens schema`."
    )]
    Report {
        /// Path to an OTLP JSON trace file.
        file: PathBuf,

        /// Trace ID to inspect.
        #[arg(long = "trace-id")]
        trace_id: String,

        /// Output HTML file path. Existing files are overwritten.
        #[arg(long = "html")]
        html: PathBuf,
    },

    /// Print the JSON output schema and field descriptions.
    #[command(
        long_about = "Print the JSON output schema and field descriptions.\n\nUse this command when an AI agent, script, CI job, or human needs to understand what `--output json` means without reading repository files directly.",
        after_help = "Examples:\n  tracelens schema --output text\n  tracelens schema --output json\n  tracelens schema --command detect --output text\n\nJSON output always prints the full schema in this version; use `$defs.<command>Output` for a command branch."
    )]
    Schema {
        /// Limit the text field reference to one command.
        #[arg(long = "command", value_enum, default_value_t = SchemaCommandFilter::All)]
        command: SchemaCommandFilter,

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum TimelineMode {
    Bar,
    Flame,
}

impl From<TimelineMode> for crate::analysis::timeline::TimelineMode {
    fn from(value: TimelineMode) -> Self {
        match value {
            TimelineMode::Bar => Self::Bar,
            TimelineMode::Flame => Self::Flame,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum SchemaCommandFilter {
    All,
    Validate,
    Summary,
    ListTraces,
    Tree,
    Services,
    CriticalPath,
    Timeline,
    Detect,
}

impl From<SchemaCommandFilter> for SchemaCommand {
    fn from(value: SchemaCommandFilter) -> Self {
        match value {
            SchemaCommandFilter::All => Self::All,
            SchemaCommandFilter::Validate => Self::Validate,
            SchemaCommandFilter::Summary => Self::Summary,
            SchemaCommandFilter::ListTraces => Self::ListTraces,
            SchemaCommandFilter::Tree => Self::Tree,
            SchemaCommandFilter::Services => Self::Services,
            SchemaCommandFilter::CriticalPath => Self::CriticalPath,
            SchemaCommandFilter::Timeline => Self::Timeline,
            SchemaCommandFilter::Detect => Self::Detect,
        }
    }
}

pub fn run() -> Result<ExitCode> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let code = exit_code::from_clap_code(error.exit_code());
            error.print()?;
            return Ok(code);
        }
    };
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
                Ok(exit_code::failure())
            } else {
                Ok(exit_code::success())
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
            Ok(exit_code::success())
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
            Ok(exit_code::success())
        }
        Commands::Detect {
            file,
            limit,
            output,
        } => {
            if limit == 0 {
                return Err(anyhow!("--limit must be greater than 0"));
            }

            let collection = load_collection(&file)?;
            ensure_has_spans(&collection)?;
            let analysis = analyze_detect(&collection, limit);

            match output {
                OutputFormat::Text => {
                    print!(
                        "{}",
                        format_detect(&file, &analysis, &collection, text_style)
                    )
                }
                OutputFormat::Json => print!("{}", format_detect_json(&analysis, &collection)),
            }
            Ok(exit_code::success())
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
            Ok(exit_code::success())
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
            Ok(exit_code::success())
        }
        Commands::Timeline {
            file,
            trace_id,
            width,
            mode,
            max_rows,
            output,
        } => {
            if !(MIN_TIMELINE_WIDTH..=MAX_TIMELINE_WIDTH).contains(&width) {
                return Err(anyhow!(
                    "--width must be between {MIN_TIMELINE_WIDTH} and {MAX_TIMELINE_WIDTH}"
                ));
            }

            let normalized_trace_id = normalize_hex_id(&trace_id, TRACE_ID_LEN)
                .map_err(|message| anyhow!("invalid --trace-id: {message}"))?;
            let collection = load_collection(&file)?;
            ensure_has_spans(&collection)?;
            let trace = collection
                .traces
                .get(&normalized_trace_id)
                .ok_or_else(|| anyhow!("trace_id not found: {normalized_trace_id}"))?;
            let critical_path = analyze_critical_path(trace);
            let timeline = analyze_timeline(trace, &critical_path, width, mode.into(), max_rows);

            match output {
                OutputFormat::Text => print!(
                    "{}",
                    format_timeline(&timeline, &critical_path, trace, text_style)
                ),
                OutputFormat::Json => {
                    print!("{}", format_timeline_json(&timeline, &critical_path, trace))
                }
            }
            Ok(exit_code::success())
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
                    format_services(
                        &analysis,
                        &trace.diagnostics,
                        &trace.cross_service_edges,
                        text_style
                    )
                ),
                OutputFormat::Json => print!("{}", format_services_json(&analysis, trace)),
            }
            Ok(exit_code::success())
        }
        Commands::Report {
            file,
            trace_id,
            html,
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
            let detect =
                analyze_detect(&collection, crate::output::html::detect_limit_for_report());
            let report = render_html_report(trace, &duration, &critical_path, &detect);
            std::fs::write(&html, report)
                .with_context(|| format!("failed to write html to {}", html.display()))?;
            println!(
                "wrote {} (trace_id: {})",
                html.display(),
                normalized_trace_id
            );
            Ok(exit_code::success())
        }
        Commands::Schema { command, output } => {
            let command = SchemaCommand::from(command);
            match output {
                OutputFormat::Text => print!("{}", format_schema_text(command)?),
                OutputFormat::Json => print!("{}", format_schema_json()),
            }
            Ok(exit_code::success())
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
