use anyhow::Result;
use home::home_dir;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use std::io::Write;
use std::time::SystemTime;
use std::{collections::HashMap, io::BufRead, path::PathBuf};

const RECENT_TIMEOUT: u64 = 60 * 60 * 24 * 30;

pub fn get_current_time() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub struct History {}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    instance_id: String,
    when: u64,
}

impl HistoryEntry {
    pub fn new(instance_id: impl Into<String>) -> HistoryEntry {
        HistoryEntry {
            instance_id: instance_id.into(),
            when: get_current_time(),
        }
    }

    pub fn get_instance_id(&self) -> &str {
        &self.instance_id
    }

    pub fn get_when(&self) -> u64 {
        self.when
    }
}
// If we ever need to save the history and read it at the same time, we need to lock the file.
impl History {
    pub fn save(entry: HistoryEntry) -> Result<()> {
        let mut entries = Self::read()?;
        entries.insert(entry.get_instance_id().to_string(), entry);
        Self::write(&entries)
    }

    pub fn read() -> Result<HashMap<String, HistoryEntry>> {
        let mut entries = HashMap::new();
        let file = Self::get_history_path()?;
        let Ok(file) = std::fs::File::open(file) else {
            return Ok(entries);
        };
        let current_time = get_current_time();
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            let entry: HistoryEntry = from_str(&line)?;
            if entry.when < current_time - RECENT_TIMEOUT {
                continue;
            }
            match entries.get_mut(entry.get_instance_id()) {
                Some(existing) => {
                    if existing.when < entry.when {
                        *existing = entry;
                    }
                }
                None => {
                    entries.insert(entry.get_instance_id().to_string(), entry);
                }
            }
        }
        Ok(entries)
    }

    fn write(entries: &HashMap<String, HistoryEntry>) -> Result<()> {
        let file = Self::get_history_path()?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)?;
        for entry in entries.values() {
            writeln!(file, "{}", to_string(entry)?)?;
        }
        file.flush()?;
        Ok(())
    }

    pub fn reset() -> Result<()> {
        let file = Self::get_history_path()?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(file)?;
        file.flush()?;
        Ok(())
    }

    fn get_history_path() -> Result<PathBuf> {
        let Some(home_dir) = home_dir() else {
            return Result::Err(anyhow::anyhow!("Could not find home directory"));
        };
        Ok(home_dir.join(".sm_connect_history"))
    }
}
