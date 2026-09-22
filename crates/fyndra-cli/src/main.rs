use clap::Parser;
use colored::*;
use fyndra_core::{CrawlerConfig, Database, SearchQuery};
use human_bytes::human_bytes;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(
    name = "fyndra-cli",
    about = "Lightning-fast file search tool for Linux inspired by Voidtools Everything",
    version = "0.1.0"
)]
struct Cli {
    /// Search query term or pattern
    #[arg(default_value = "")]
    query: String,

    /// Root directories to index (defaults to user $HOME)
    #[arg(short, long, value_delimiter = ',')]
    paths: Option<Vec<PathBuf>>,

    /// Enable regular expression search
    #[arg(short, long)]
    regex: bool,

    /// Enable wildcard / glob pattern search (* and ?)
    #[arg(short, long)]
    wildcard: bool,

    /// Case sensitive search
    #[arg(short = 'c', long)]
    case_sensitive: bool,

    /// Match directories only
    #[arg(short, long, visible_alias = "dirs")]
    dirs_only: bool,

    /// Match files only
    #[arg(short = 'F', long, visible_alias = "files")]
    files_only: bool,

    /// Maximum number of search results to display
    #[arg(short, long, default_value_t = 50)]
    limit: usize,

    /// Output results as raw JSON
    #[arg(short, long)]
    json: bool,

    /// Show benchmarking statistics
    #[arg(short, long)]
    benchmark: bool,
}

fn main() {
    let cli = Cli::parse();

    // TTY-aware colors: disable ANSI when piped, NO_COLOR set, or --json (fd/eza pattern)
    // Inspired by sharkdp/fd (MIT/Apache): is_terminal + NO_COLOR + --json
    if std::env::var("NO_COLOR").is_ok() || !std::io::stdout().is_terminal() || cli.json {
        colored::control::set_override(false);
    }

    // Default to HOME directory, or root "/" if HOME is unset
    let scan_paths = cli.paths.unwrap_or_else(|| {
        if let Ok(home) = std::env::var("HOME") {
            vec![PathBuf::from(home)]
        } else {
            vec![PathBuf::from("/")]
        }
    });

    if !cli.json {
        eprintln!(
            "{} Indexing {:?}...",
            "⚡".yellow(),
            scan_paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
        );
    }

    let start_index = Instant::now();
    let db = Database::new(CrawlerConfig::default());
    let stats = db.index_roots(&scan_paths);
    let index_duration = start_index.elapsed();

    if !cli.json {
        eprintln!(
            "{} Indexed {} files & {} directories in {:.2?} (RAM: ~{:.1} MB)",
            "✔".green().bold(),
            stats.total_files.to_string().cyan(),
            stats.total_dirs.to_string().cyan(),
            index_duration,
            (db.memory_usage_bytes() as f64) / 1_048_576.0
        );
    }

    let search_query = SearchQuery {
        raw: cli.query.clone(),
        case_sensitive: cli.case_sensitive,
        is_regex: cli.regex,
        is_wildcard: cli.wildcard,
        dir_only: cli.dirs_only,
        file_only: cli.files_only,
        category: fyndra_core::FileCategory::All,
        max_results: cli.limit,
    };

    let result = db.search(&search_query);

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&result.entries).unwrap());
        return;
    }

    println!();
    println!(
        "{} Found {} matches in {:.3} ms (showing max {}):",
        "🔍".blue(),
        result.entries.len().to_string().green().bold(),
        (result.elapsed_micros as f64) / 1000.0,
        cli.limit
    );
    println!("{}", "─".repeat(80).dimmed());

    for entry in &result.entries {
        let icon = if entry.is_dir { "📁" } else { "📄" };
        let type_label = if entry.is_dir {
            "<DIR>".blue().bold()
        } else {
            human_bytes(entry.size as f64).dimmed()
        };

        let path_str = entry.parent.display().to_string();
        println!("{} {:<35} {:>10}  {}", icon, entry.name.bold(), type_label, path_str.dimmed());
    }

    println!("{}", "─".repeat(80).dimmed());
    if cli.benchmark {
        println!(
            "{} Scanned {} total items in RAM in {:.2?} ms",
            "⏱".magenta(),
            result.total_scanned,
            (result.elapsed_micros as f64) / 1000.0
        );
    }
}
