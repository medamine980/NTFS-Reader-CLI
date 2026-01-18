use anyhow::{Context, Result};
use ntfs_reader::journal::{Journal, JournalOptions, NextUsn};
use ntfs_reader::volume::Volume;
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;
use std::io::Write;

use crate::OutputFormat;

#[derive(Debug, Serialize, Deserialize)]
pub struct JournalEvent {
    pub usn: i64,
    pub timestamp_ms: u128,
    pub file_id: String,
    pub parent_id: String,
    pub reason: u32,
    pub reason_str: String,
    pub path: String,
}

impl JournalEvent {
    fn from_usn_record(record: &ntfs_reader::journal::UsnRecord) -> Self {
        JournalEvent {
            usn: record.usn,
            timestamp_ms: record.timestamp.as_millis(),
            file_id: format!("{:?}", record.file_id),
            parent_id: format!("{:?}", record.parent_id),
            reason: record.reason,
            reason_str: Journal::get_reason_str(record.reason),
            path: record.path.to_string_lossy().to_string(),
        }
    }
}

fn normalize_volume_path(volume: &str) -> String {
    let volume = volume.trim();
    
    // If it's just a drive letter, convert to extended path
    if volume.len() == 2 && volume.chars().nth(1) == Some(':') {
        return format!("\\\\?\\{}:", volume.chars().nth(0).unwrap());
    }
    
    // If it's a drive letter with backslash, remove it
    if volume.len() == 3 && volume.ends_with(":\\") {
        return format!("\\\\?\\{}:", volume.chars().nth(0).unwrap());
    }
    
    // Return as-is if already in extended format
    volume.to_string()
}

pub fn monitor_journal(
    volume: &str,
    from_start: bool,
    from_usn: Option<i64>,
    reason_mask: Option<u32>,
    max_events: Option<usize>,
    continuous: bool,
    output: OutputFormat,
) -> Result<()> {
    let volume_path = normalize_volume_path(volume);
    
    eprintln!("Opening volume: {}", volume_path);
    let vol = Volume::new(&volume_path)
        .context("Failed to open volume. Make sure you're running as Administrator.")?;
    
    let next_usn = if from_start {
        NextUsn::First
    } else if let Some(usn) = from_usn {
        NextUsn::Custom(usn)
    } else {
        NextUsn::Next
    };
    
    let options = JournalOptions {
        reason_mask: reason_mask.unwrap_or(0xFFFFFFFF),
        next_usn,
        max_history_size: ntfs_reader::journal::HistorySize::Limited(1000),
    };
    
    eprintln!("Opening USN journal...");
    let mut journal = Journal::new(vol, options)
        .context("Failed to open USN journal")?;
    
    let mut all_events = Vec::new();
    let mut total_read = 0;
    
    loop {
        eprintln!("Reading journal events...");
        let events = journal.read()
            .context("Failed to read journal events")?;
        
        if events.is_empty() {
            if !continuous {
                eprintln!("No more events available.");
                break;
            }
            eprintln!("No new events, waiting...");
            thread::sleep(Duration::from_millis(500));
            continue;
        }
        
        eprintln!("Read {} events", events.len());
        
        for event in events {
            let journal_event = JournalEvent::from_usn_record(&event);
            
            if continuous {
                // Output each event immediately in continuous mode
                match output {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string(&journal_event)?);
                    }
                    OutputFormat::JsonPretty => {
                        println!("{}", serde_json::to_string_pretty(&journal_event)?);
                    }
                    OutputFormat::Bincode => {
                        let encoded = bincode::serialize(&journal_event)?;
                        std::io::stdout().write_all(&encoded)?;
                        std::io::stdout().flush()?;
                    }
                    OutputFormat::Msgpack => {
                        let mut buf = Vec::new();
                        rmp_serde::encode::write(&mut buf, &journal_event)?;
                        std::io::stdout().write_all(&buf)?;
                        std::io::stdout().flush()?;
                    }
                    OutputFormat::Csv => {
                        if total_read == 0 {
                            output_csv_header()?;
                        }
                        output_csv_event(&journal_event)?;
                    }
                }
            } else {
                all_events.push(journal_event);
            }
            
            total_read += 1;
            
            if let Some(max) = max_events {
                if total_read >= max {
                    eprintln!("Reached maximum event limit: {}", max);
                    if !continuous {
                        output_events(&all_events, output)?;
                    }
                    return Ok(());
                }
            }
        }
        
        if !continuous {
            // In non-continuous mode, try one more time to get any remaining events
            let remaining = journal.read().context("Failed to read journal events")?;
            if remaining.is_empty() {
                break;
            }
            
            for event in &remaining {
                let journal_event = JournalEvent::from_usn_record(event);
                all_events.push(journal_event);
                total_read += 1;
                
                if let Some(max) = max_events {
                    if total_read >= max {
                        break;
                    }
                }
            }
        }
    }
    
    if !continuous && !all_events.is_empty() {
        output_events(&all_events, output)?;
    }
    
    Ok(())
}

fn output_events(events: &[JournalEvent], output: OutputFormat) -> Result<()> {
    match output {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(&events)?);
        }
        OutputFormat::JsonPretty => {
            println!("{}", serde_json::to_string_pretty(&events)?);
        }
        OutputFormat::Bincode => {
            let encoded = bincode::serialize(&events)?;
            std::io::stdout().write_all(&encoded)?;
        }
        OutputFormat::Msgpack => {
            let mut buf = Vec::new();
            rmp_serde::encode::write(&mut buf, &events)?;
            std::io::stdout().write_all(&buf)?;
        }
        OutputFormat::Csv => {
            output_csv_header()?;
            for event in events {
                output_csv_event(event)?;
            }
        }
    }
    Ok(())
}

fn output_csv_header() -> Result<()> {
    println!("usn,timestamp_ms,file_id,parent_id,reason,reason_str,path");
    Ok(())
}

fn output_csv_event(event: &JournalEvent) -> Result<()> {
    println!(
        "{},{},{},{},{},{},{}",
        event.usn,
        event.timestamp_ms,
        escape_csv(&event.file_id),
        escape_csv(&event.parent_id),
        event.reason,
        escape_csv(&event.reason_str),
        escape_csv(&event.path)
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
