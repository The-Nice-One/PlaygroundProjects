use crate::PROJECT_DIRECTORY;
use chrono::prelude::*;
use flexi_logger::{DeferredNow, FileSpec, Logger, Record};
use indexmap::IndexMap;
use log::*;
use std::sync::{Arc, LazyLock, Mutex};

pub fn log_format(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "{}: {} ({})",
        record.level(),
        &record.args(),
        now.format("%Y-%m-%d %H:%M:%S%.6f %:z"),
    )
}

#[macro_export]
macro_rules! logger {
    () => {
        LOGGER.lock().unwrap()
    };
}

#[derive(PartialEq, Eq, Hash)]
pub enum EntryId {
    Configuration,
    DiscordPresence,
    Player,
}

pub static LOGGER: LazyLock<Arc<Mutex<Log<EntryId>>>> = LazyLock::new(|| {
    let log_directory = PROJECT_DIRECTORY.data_dir().join("log_files");
    Logger::try_with_str("info")
        .unwrap()
        .log_to_file(FileSpec::default().directory(log_directory))
        .format_for_files(log_format)
        .start()
        .unwrap();

    Arc::new(Mutex::new(Log::new()))
});

pub struct Log<K>
where
    K: Eq + std::hash::Hash,
{
    pub entries: IndexMap<K, Entry>,
}

impl<K> Log<K>
where
    K: Eq + std::hash::Hash,
{
    pub fn new() -> Self {
        Self {
            entries: IndexMap::new(),
        }
    }

    pub fn add_entry(&mut self, entry_key: K, entry: Entry) {
        self.entries.insert(entry_key, entry);
    }

    pub fn get_entry(&mut self, entry_key: K) -> Option<&mut Entry> {
        self.entries.get_mut(&entry_key)
    }
}

pub enum EntryStatus {
    Incomplete,
    Complete,
    Failed,
}

pub struct Entry {
    pub status: EntryStatus,
    pub description: String,
    pub timestamp: DateTime<Local>,
}

impl Entry {
    pub fn new<S: Into<String>>(description: S) -> Self {
        let description = description.into();
        info!("{}", description);
        Self {
            status: EntryStatus::Incomplete,
            description,
            timestamp: Local::now(),
        }
    }
    pub fn new_complete<S: Into<String>>(description: S) -> Self {
        let description = description.into();
        info!("{}", description);
        Self {
            status: EntryStatus::Complete,
            description,
            timestamp: Local::now(),
        }
    }
    pub fn complete<S: Into<String>>(&mut self, description: S) {
        let description = description.into();
        info!("{}", description);
        self.status = EntryStatus::Complete;
        self.description = description;
        self.timestamp = Local::now();
    }
    pub fn fail<S: Into<String>>(&mut self, description: S) {
        let description = description.into();
        warn!("{}", description);
        self.status = EntryStatus::Failed;
        self.description = description;
        self.timestamp = Local::now();
    }
}
