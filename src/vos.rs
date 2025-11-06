use sha3::{Digest, Keccak256};
use std::fs;
// use std::io::Read; // TODO: Enable for streaming reads in future versions
use std::path::Path;
use crate::objects::{self, ObjectId};
use serde::Serialize;
// use fastcdc::v2020::{StreamCDC, FastCDC}; // TODO: Enable for advanced chunking in future versions

/// Hashes raw byte data using the SHA-3 (Keccak-256) PQC-resistant algorithm.
pub fn hash_data(data: &[u8]) -> ObjectId {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    
    // Convert the 32-byte hash result into a hex string.
    format!("{:x}", hasher.finalize())
}

/// Serializes an object (like Commit or Directory) and returns its hash (ID).
pub fn hash_object<T: Serialize>(object: &T) -> Result<ObjectId, serde_json::Error> {
    // For simplicity in the MVP, we'll use JSON serialization. 
    // A production version would use a more compact binary format (like BSON or custom)
    // for superior speed.
    let serialized_data = serde_json::to_vec(object)?; 
    
    Ok(hash_data(&serialized_data))
}

// Example placeholder for saving an object to the file system (by its hash)
/// Chunks a file's content and returns the ID of the root object (File object ID) 
/// that represents the content. This automatically saves all new chunks to VOS.
pub fn chunk_and_save_file(path: &Path) -> Result<(ObjectId, usize), std::io::Error> {
    // For MVP v0.2, we'll simplify chunking by treating each file as a single chunk
    // This ensures the revert functionality works correctly while maintaining the VOS architecture
    let file_content = fs::read(path)?;
    let size = file_content.len();
    
    // Save the entire file content as a single chunk
    let chunk_id = save_object(&file_content);
    
    // Create and save the File object that references this chunk
    let file_object = objects::File {
        root_chunk_id: chunk_id.clone(),
        size,
    };
    
    let file_id = hash_object(&file_object).unwrap();
    save_object(&serde_json::to_vec(&file_object).unwrap()); // Save the File object metadata
    
    Ok((file_id, size))
}

/// Saves raw data to the VOS object store by its hash ID.
/// This is a simplified version for MVP - in production this would handle
/// directory structure and deduplication more efficiently.
pub fn save_object(data: &[u8]) -> ObjectId {
    let object_id = hash_data(data);
    
    // Create object storage path: .orb/objects/ab/cdef123...
    let objects_dir = std::path::Path::new(".orb").join("objects");
    let (prefix, suffix) = object_id.split_at(2);
    let object_dir = objects_dir.join(prefix);
    let object_file = object_dir.join(suffix);
    
    // Create directory if it doesn't exist
    if let Err(_) = fs::create_dir_all(&object_dir) {
        eprintln!("Warning: Could not create object directory");
        return object_id;
    }
    
    // Write the object data if it doesn't already exist (deduplication)
    if !object_file.exists() {
        if let Err(_) = fs::write(&object_file, data) {
            eprintln!("Warning: Could not save object {}", object_id);
        }
    }
    
    object_id
}

/// Stores object data with a pre-computed ID (for objects received from server)
pub fn store_object_with_id(object_id: &str, data: &[u8]) -> Result<(), std::io::Error> {
    let objects_dir = Path::new(".orb").join("objects");
    let (prefix, suffix) = object_id.split_at(2);
    let object_dir = objects_dir.join(prefix);
    let object_file = object_dir.join(suffix);
    
    // Create directory if it doesn't exist
    fs::create_dir_all(&object_dir)?;
    
    // Write the object data (overwrites if exists, for sync consistency)
    fs::write(&object_file, data)?;
    
    Ok(())
}