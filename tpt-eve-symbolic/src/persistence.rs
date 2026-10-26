// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::working_memory::WorkingMemory;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::PathBuf;
use tpt_eve_core::Fact;

const FACTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("facts");

/// Persistence layer for working memory using redb.
pub struct WorkingMemoryStore {
    db: Database,
}

impl WorkingMemoryStore {
    pub fn new(path: PathBuf) -> Result<Self, tpt_eve_core::EveError> {
        let db = Database::create(&path).map_err(|e| {
            tpt_eve_core::EveError::Persistence(format!("Failed to create database: {}", e))
        })?;
        Ok(WorkingMemoryStore { db })
    }

    /// Load working memory from the store.
    pub fn load(&self) -> Result<WorkingMemory, tpt_eve_core::EveError> {
        let tx = self
            .db
            .begin_read()
            .map_err(|e| tpt_eve_core::EveError::Persistence(e.to_string()))?;

        let table = match tx.open_table(FACTS_TABLE) {
            Ok(t) => t,
            Err(_) => return Ok(WorkingMemory::new()), // No data yet
        };

        let mut wm = WorkingMemory::new();
        for item in table
            .iter()
            .map_err(|e| tpt_eve_core::EveError::Persistence(e.to_string()))?
        {
            let (_, data) = item.map_err(|e| tpt_eve_core::EveError::Persistence(e.to_string()))?;
            let fact: Fact = serde_json::from_slice(data.value()).map_err(|e| {
                tpt_eve_core::EveError::Persistence(format!("Failed to deserialize fact: {}", e))
            })?;
            wm.add_fact(fact);
        }
        Ok(wm)
    }

    /// Save working memory to the store.
    pub fn save(&self, wm: &WorkingMemory) -> Result<(), tpt_eve_core::EveError> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| tpt_eve_core::EveError::Persistence(e.to_string()))?;

        {
            let mut table = write_txn
                .open_table(FACTS_TABLE)
                .map_err(|e| tpt_eve_core::EveError::Persistence(e.to_string()))?;

            for (i, fact) in wm.facts().iter().enumerate() {
                let data = serde_json::to_vec(fact).map_err(|e| {
                    tpt_eve_core::EveError::Persistence(format!("Failed to serialize fact: {}", e))
                })?;
                table
                    .insert(i.to_string().as_str(), data.as_slice())
                    .map_err(|e| tpt_eve_core::EveError::Persistence(e.to_string()))?;
            }
        }

        write_txn
            .commit()
            .map_err(|e| tpt_eve_core::EveError::Persistence(e.to_string()))?;
        Ok(())
    }
}

/// The user's home directory, from the environment. `USERPROFILE` is tried first on Windows and
/// `HOME` elsewhere, so the crate needs no path-lookup dependency.
fn home_dir() -> Option<PathBuf> {
    let vars: &[&str] = if cfg!(windows) {
        &["USERPROFILE", "HOME"]
    } else {
        &["HOME"]
    };
    vars.iter()
        .filter_map(std::env::var_os)
        .find(|value| !value.is_empty())
        .map(PathBuf::from)
}

impl Default for WorkingMemoryStore {
    fn default() -> Self {
        // Use default path in user's home directory
        let path = home_dir()
            .map(|h| h.join(".tpt").join("eve").join("working_memory.redb"))
            .unwrap_or_else(|| PathBuf::from(".tpt_eve_working_memory.redb"));

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Warning: could not create {}: {e}", parent.display());
            }
        }

        match Self::new(path) {
            Ok(store) => store,
            Err(e) => {
                eprintln!("Error: {e}");
                eprintln!(
                    "Could not open default working memory store. \
                     Run `eve extract --save <file>` to create a session first."
                );
                std::process::exit(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn persist_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.redb");
        let store = WorkingMemoryStore::new(path).unwrap();

        let mut original = WorkingMemory::new();
        original.add_fact(Fact::asserted(
            tpt_eve_core::Value::Symbol("TypeScript".into()),
            tpt_eve_core::Value::Symbol("is".into()),
            tpt_eve_core::Value::Text("programming language".into()),
        ));
        store.save(&original).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.facts().len(), 1);
    }
}
