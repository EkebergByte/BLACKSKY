use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use crate::models::{BlackskyError, SnapshotRecord};

const CSV_HEADER: &str = "date,timestamp_utc,chain,block_number,address,balance_geod\n";

fn validate_record(record: &SnapshotRecord) -> Result<(), BlackskyError> {
    if !record.balance_geod.is_finite() || record.balance_geod < 0.0 {
        return Err(BlackskyError::ValidationError("Invalid balance value".to_string()));
    }
    let fields = [
        &record.date,
        &record.chain,
        &record.address,
        &record.category,
        &record.label,
    ];
    for field in fields {
        if field.contains(',') || field.contains('\n') || field.contains('\r') {
            return Err(BlackskyError::ValidationError("Invalid character in text field".to_string()));
        }
    }
    Ok(())
}

fn normalize_address(address: &str, chain: &str) -> String {
    match chain {
        "Polygon" => address.to_lowercase(),
        _ => address.to_string(),
    }
}

pub fn append_snapshot_records(file_path: &str, records: &[SnapshotRecord]) -> Result<usize, BlackskyError> {
    let path = Path::new(file_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| BlackskyError::StorageError(e.to_string()))?;
    }

    let mut existing_keys = HashSet::new();
    let mut needs_reset = false;
    
    if path.exists() {
        let file = File::open(file_path).map_err(|e| BlackskyError::StorageError(e.to_string()))?;
        let mut reader = BufReader::new(file);
        let mut first_line = String::new();
        reader.read_line(&mut first_line).map_err(|e| BlackskyError::StorageError(e.to_string()))?;
        
        needs_reset = first_line.trim() != CSV_HEADER.trim();
        
        if !needs_reset {
            for line in reader.lines() {
                let line = line.map_err(|e| BlackskyError::StorageError(e.to_string()))?;
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() == 6 {
                    existing_keys.insert((
                        parts[0].to_string(),
                        parts[2].to_string(),
                        normalize_address(parts[4], parts[2]),
                    ));
                }
            }
        }
    }

    let file = OpenOptions::new()
        .write(true)
        .truncate(needs_reset)
        .create(true)
        .append(!needs_reset)
        .open(file_path)
        .map_err(|e| BlackskyError::StorageError(e.to_string()))?;

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .map_err(|e| BlackskyError::StorageError(e.to_string()))?;
    let mut writer = BufWriter::new(file);

    if !file_has_content {
        writer
            .write_all(CSV_HEADER.as_bytes())
            .map_err(|e| BlackskyError::StorageError(e.to_string()))?;
    }

    let mut inserted_count = 0;
    for record in records {
        validate_record(record)?;
        let normalized_addr = normalize_address(&record.address, &record.chain);
        let key = (record.date.clone(), normalized_addr.clone());

        if !existing_keys.contains(&key) {
            let line = format!(
                "{},{},{},{},{},{},{},{},{:.4}\n",
                record.date,
                record.timestamp_utc,
                record.chain,
                record.block_number,
                normalized_addr,
                record.category,
                record.label,
                record.is_liquid_overhang,
                record.balance_geod
            );
            writer
                .write_all(line.as_bytes())
                .map_err(|e| BlackskyError::StorageError(e.to_string()))?;
            existing_keys.insert(key);
            inserted_count += 1;
        }
    }

    writer.flush().map_err(|e| BlackskyError::StorageError(e.to_string()))?;
    Ok(inserted_count)
}
