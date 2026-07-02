use crate::working_memory::WorkingMemory;
use eve_core::Fact;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::PathBuf;

const FACTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("facts");

/// Persistence layer for working memory using redb.
pub struct WorkingMemoryStore {
    db: Database,
}

impl WorkingMemoryStore {
    pub fn new(path: PathBuf) -> Result<Self, eve_core::EveError> {
        let db = Database::create(&path).map_err(|e| {
            eve_core::EveError::Persistence(format!("Failed to create database: {}", e))
        })?;
        Ok(WorkingMemoryStore { db })
    }

    /// Load working memory from the store.
    pub fn load(&self) -> Result<WorkingMemory, eve_core::EveError> {
        let tx = self
            .db
            .begin_read()
            .map_err(|e| eve_core::EveError::Persistence(e.to_string()))?;

        let table = match tx.open_table(FACTS_TABLE) {
            Ok(t) => t,
            Err(_) => return Ok(WorkingMemory::new()), // No data yet
        };

        let mut wm = WorkingMemory::new();
        for item in table
            .iter()
            .map_err(|e| eve_core::EveError::Persistence(e.to_string()))?
        {
            let (_, data) = item.map_err(|e| eve_core::EveError::Persistence(e.to_string()))?;
            let fact: Fact = serde_json::from_slice(data.value()).map_err(|e| {
                eve_core::EveError::Persistence(format!("Failed to deserialize fact: {}", e))
            })?;
            wm.add_fact(fact);
        }
        Ok(wm)
    }

    /// Save working memory to the store.
    pub fn save(&self, wm: &WorkingMemory) -> Result<(), eve_core::EveError> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| eve_core::EveError::Persistence(e.to_string()))?;

        {
            let mut table = write_txn
                .open_table(FACTS_TABLE)
                .map_err(|e| eve_core::EveError::Persistence(e.to_string()))?;

            for (i, fact) in wm.facts().iter().enumerate() {
                let data = serde_json::to_vec(fact).map_err(|e| {
                    eve_core::EveError::Persistence(format!("Failed to serialize fact: {}", e))
                })?;
                table
                    .insert(i.to_string().as_str(), data.as_slice())
                    .map_err(|e| eve_core::EveError::Persistence(e.to_string()))?;
            }
        }

        write_txn
            .commit()
            .map_err(|e| eve_core::EveError::Persistence(e.to_string()))?;
        Ok(())
    }
}

impl Default for WorkingMemoryStore {
    fn default() -> Self {
        // Use default path in user's home directory
        let path = dirs::home_dir()
            .map(|h| h.join(".tpt").join("eve").join("working_memory.redb"))
            .unwrap_or_else(|| PathBuf::from(".tpt_eve_working_memory.redb"));

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        Self::new(path).expect("Failed to create default store")
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
            eve_core::Value::Symbol("TypeScript".into()),
            eve_core::Value::Symbol("is".into()),
            eve_core::Value::Text("programming language".into()),
        ));
        store.save(&original).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.facts().len(), 1);
    }
}
