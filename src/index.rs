use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;
use serde::{Deserialize, Serialize};
use crate::objects::ObjectId;

/// Represents a single file entry in the VOS Index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub path: String,
    pub mtime: u64,        // Modified time in seconds since UNIX epoch
    pub size: u64,         // File size in bytes
    pub file_id: ObjectId, // The File object ID from VOS
}

/// The VOS Index - tracks metadata of all files in the last saved snapshot
#[derive(Debug, Serialize, Deserialize)]
pub struct VosIndex {
    pub version: u32,
    pub entries: HashMap<String, IndexEntry>,
}

impl VosIndex {
    /// Creates a new empty VOS Index
    pub fn new() -> Self {
        Self {
            version: 1,
            entries: HashMap::new(),
        }
    }

    /// Loads the VOS Index from disk, or creates a new one if it doesn't exist
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let index_path = Path::new(".orb").join("index");
        
        if !index_path.exists() {
            return Ok(Self::new());
        }

        let data = fs::read_to_string(index_path)?;
        let index: VosIndex = serde_json::from_str(&data)?;
        Ok(index)
    }

    /// Saves the VOS Index to disk
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let index_path = Path::new(".orb").join("index");
        let data = serde_json::to_string_pretty(self)?;
        fs::write(index_path, data)?;
        Ok(())
    }

    /// Updates or adds an entry in the index
    pub fn update_entry(&mut self, path: String, mtime: u64, size: u64, file_id: ObjectId) {
        let entry = IndexEntry {
            path: path.clone(),
            mtime,
            size,
            file_id,
        };
        self.entries.insert(path, entry);
    }

    /// Removes an entry from the index
    #[allow(dead_code)]
    pub fn remove_entry(&mut self, path: &str) {
        self.entries.remove(path);
    }

    /// Gets file metadata for comparison
    pub fn get_file_metadata(file_path: &Path) -> Result<(u64, u64), Box<dyn std::error::Error>> {
        let metadata = fs::metadata(file_path)?;
        let mtime = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        let size = metadata.len();
        Ok((mtime, size))
    }

    /// Checks if a file has changed compared to the index
    pub fn has_file_changed(&self, path: &str, file_path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
        // If file is not in index, it's new/untracked
        let Some(entry) = self.entries.get(path) else {
            return Ok(true);
        };

        // Check if file still exists
        if !file_path.exists() {
            return Ok(true); // File was deleted
        }

        // Compare metadata
        let (current_mtime, current_size) = Self::get_file_metadata(file_path)?;
        
        // If timestamp or size changed, file might be modified
        Ok(entry.mtime != current_mtime || entry.size != current_size)
    }

    /// Gets all tracked file paths
    #[allow(dead_code)]
    pub fn get_tracked_paths(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    /// Clears all entries (for fresh rebuild)
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}