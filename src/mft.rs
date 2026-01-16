use anyhow::{Context, Result};
use ntfs_reader::file_info::FileInfo;
use ntfs_reader::mft::Mft;
use ntfs_reader::volume::Volume;
use serde::{Deserialize, Serialize};
use regex::Regex;

use crate::OutputFormat;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileRecord {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub accessed: Option<String>,
}

impl FileRecord {
    fn from_file_info(info: &FileInfo) -> Self {
        FileRecord {
            name: info.name.clone(),
            path: info.path.to_string_lossy().to_string(),
            is_directory: info.is_directory,
            size: info.size,
            created: info.created.map(|t| format_time(t)),
            modified: info.modified.map(|t| format_time(t)),
            accessed: info.accessed.map(|t| format_time(t)),
        }
    }
}

fn format_time(time: time::OffsetDateTime) -> String {
    time.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| time.to_string())
}

fn normalize_volume_path(volume: &str) -> String {
    let volume = volume.trim();
    
    // If it's just a drive letter, convert to device path
    if volume.len() == 2 && volume.chars().nth(1) == Some(':') {
        return format!("\\\\.\\{}:", volume.chars().nth(0).unwrap());
    }
    
    // If it's a drive letter with backslash, remove it
    if volume.len() == 3 && volume.ends_with(":\\") {
        return format!("\\\\.\\{}:", volume.chars().nth(0).unwrap());
    }
    
    // Return as-is if already in device format
    volume.to_string()
}

pub fn list_files(
    volume: &str,
    filter: Option<&str>,
    directories_only: bool,
    limit: Option<usize>,
    output: OutputFormat,
) -> Result<()> {
    let volume_path = normalize_volume_path(volume);
    
    eprintln!("Opening volume: {}", volume_path);
    let vol = Volume::new(&volume_path)
        .context("Failed to open volume. Make sure you're running as Administrator.")?;
    
    eprintln!("Loading MFT...");
    let mft = Mft::new(vol).context("Failed to load MFT")?;
    
    eprintln!("Iterating files...");
    let mut records = Vec::new();
    
    // Compile regex if filter looks like a pattern or regex
    let filter_regex = filter.and_then(|f| {
        // Convert glob patterns like *.pdf to regex
        let pattern = if f.contains('*') || f.contains('?') {
            let regex_pattern = f
                .replace('\\', "\\\\")
                .replace('.', "\\.")
                .replace('*', ".*")
                .replace('?', ".")
                .to_lowercase();
            Some(regex_pattern)
        } else if f.starts_with('^') || f.contains('[') || f.contains('(') {
            // Looks like regex
            Some(f.to_lowercase())
        } else {
            // Simple substring search
            None
        };
        
        pattern.and_then(|p| Regex::new(&p).ok())
    });
    
    let filter_simple = if filter_regex.is_none() {
        filter.map(|f| f.to_lowercase())
    } else {
        None
    };

    mft.iterate_files(|file| {
        let info = FileInfo::new(&mft, file);
        
        // Apply filters
        if directories_only && !info.is_directory {
            return;
        }
        
        // Apply filter (regex or simple substring)
        if let Some(ref regex) = filter_regex {
            let path_lower = info.path.to_string_lossy().to_lowercase();
            if !regex.is_match(&path_lower) {
                return;
            }
        } else if let Some(ref filter_str) = filter_simple {
            let path_lower = info.path.to_string_lossy().to_lowercase();
            if !path_lower.contains(filter_str) {
                return;
            }
        }
        
        records.push(FileRecord::from_file_info(&info));
        
        if let Some(lim) = limit {
            if records.len() >= lim {
                return;
            }
        }
    });

    output_records(&records, output)?;
    
    Ok(())
}

pub fn file_info(volume: &str, record_number: u64, output: OutputFormat) -> Result<()> {
    let volume_path = normalize_volume_path(volume);
    
    eprintln!("Opening volume: {}", volume_path);
    let vol = Volume::new(&volume_path)
        .context("Failed to open volume. Make sure you're running as Administrator.")?;
    
    eprintln!("Loading MFT...");
    let mft = Mft::new(vol).context("Failed to load MFT")?;
    
    let file = mft
        .get_record(record_number)
        .context(format!("Record {} not found or invalid", record_number))?;
    
    let info = FileInfo::new(&mft, &file);
    let record = FileRecord::from_file_info(&info);
    
    match output {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(&record)?);
        }
        OutputFormat::JsonPretty => {
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        OutputFormat::Csv => {
            output_csv_header()?;
            output_csv_record(&record)?;
        }
    }
    
    Ok(())
}

fn output_records(records: &[FileRecord], output: OutputFormat) -> Result<()> {
    match output {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(&records)?);
        }
        OutputFormat::JsonPretty => {
            println!("{}", serde_json::to_string_pretty(&records)?);
        }
        OutputFormat::Csv => {
            output_csv_header()?;
            for record in records {
                output_csv_record(record)?;
            }
        }
    }
    Ok(())
}

fn output_csv_header() -> Result<()> {
    println!("name,path,is_directory,size,created,modified,accessed");
    Ok(())
}

fn output_csv_record(record: &FileRecord) -> Result<()> {
    println!(
        "{},{},{},{},{},{},{}",
        escape_csv(&record.name),
        escape_csv(&record.path),
        record.is_directory,
        record.size,
        record.created.as_deref().unwrap_or(""),
        record.modified.as_deref().unwrap_or(""),
        record.accessed.as_deref().unwrap_or("")
    );
    Ok(())
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
