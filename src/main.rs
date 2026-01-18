use clap::{Parser, Subcommand};
use anyhow::Result;

mod mft;
mod journal;

#[derive(Parser)]
#[command(name = "ntfs-reader-cli")]
#[command(about = "Command-line interface for NTFS MFT and USN Journal reading", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all files from the MFT
    ListFiles {
        /// Volume path (e.g., \\.\C: or C:)
        #[arg(short, long)]
        volume: String,

        /// Filter by path pattern (case-insensitive substring match)
        #[arg(short, long)]
        filter: Option<String>,

        /// Only show directories
        #[arg(short, long)]
        directories_only: bool,

        /// Limit number of results
        #[arg(short, long)]
        limit: Option<usize>,

        /// Output format
        #[arg(short, long, default_value = "json")]
        output: OutputFormat,
    },

    /// Monitor USN journal for file system changes
    Journal {
        /// Volume path (e.g., \\?\C: or C:)
        #[arg(short, long)]
        volume: String,

        /// Start from beginning of journal (default: start from current position)
        #[arg(short, long)]
        from_start: bool,

        /// Start from specific USN
        #[arg(short = 'u', long)]
        from_usn: Option<i64>,

        /// Filter by reason mask (bitmask of USN_REASON_* values)
        #[arg(short, long)]
        reason_mask: Option<u32>,

        /// Maximum number of events to read (default: read all available)
        #[arg(short, long)]
        max_events: Option<usize>,

        /// Continuously monitor for new events
        #[arg(short, long)]
        continuous: bool,

        /// Output format
        #[arg(short, long, default_value = "json")]
        output: OutputFormat,
    },

    /// Get information about a specific file by MFT record number
    FileInfo {
        /// Volume path (e.g., \\.\C: or C:)
        #[arg(short, long)]
        volume: String,

        /// MFT record number
        #[arg(short, long)]
        record: u64,

        /// Output format
        #[arg(short, long, default_value = "json")]
        output: OutputFormat,
    },
}

#[derive(Clone, Copy, Debug)]
enum OutputFormat {
    Json,
    JsonPretty,
    Csv,
    Bincode,
    Msgpack,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "json-pretty" | "pretty" => Ok(OutputFormat::JsonPretty),
            "csv" => Ok(OutputFormat::Csv),
            "bincode" | "bin" => Ok(OutputFormat::Bincode),
            "msgpack" | "messagepack" | "mp" => Ok(OutputFormat::Msgpack),
            _ => Err(format!("Invalid output format: {}", s)),
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ListFiles {
            volume,
            filter,
            directories_only,
            limit,
            output,
        } => {
            mft::list_files(&volume, filter.as_deref(), directories_only, limit, output)?;
        }
        Commands::Journal {
            volume,
            from_start,
            from_usn,
            reason_mask,
            max_events,
            continuous,
            output,
        } => {
            journal::monitor_journal(
                &volume,
                from_start,
                from_usn,
                reason_mask,
                max_events,
                continuous,
                output,
            )?;
        }
        Commands::FileInfo {
            volume,
            record,
            output,
        } => {
            mft::file_info(&volume, record, output)?;
        }
    }

    Ok(())
}
