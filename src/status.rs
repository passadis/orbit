use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::objects::{ObjectId, Commit, Directory}; // DirectoryEntry for future use
use crate::vos;
// use crate::repo; // TODO: May be needed for advanced status operations
use crate::index::VosIndex;

/// Represents the status of a file in the working directory
#[derive(Debug, PartialEq)]
pub enum FileStatus {
    Modified,
    Untracked,
    Deleted,
}

/// Fast status check using VOS Index for optimal performance
pub fn check_status() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 Orbit Status (orb check) - v0.3 with Git Interop\n");
    
    // 1. Load the VOS Index
    let index = VosIndex::load()?;
    
    if index.entries.is_empty() {
        println!("📝 Repository is empty (no commits yet)");
        println!("\nTo create your first commit, use: orb save -m \"Initial commit\"");
        return Ok(());
    }

    // 2. Fast scan: Check all indexed files for changes using metadata
    let mut changes = Vec::new();
    let mut files_needing_full_check = Vec::new();
    
    // Check tracked files
    for (path, _entry) in &index.entries {
        let file_path = Path::new(path);
        
        if !file_path.exists() {
            // File was deleted
            changes.push((path.clone(), FileStatus::Deleted));
        } else {
            // Quick metadata comparison
            match index.has_file_changed(path, file_path) {
                Ok(true) => {
                    // Metadata changed, need full check
                    files_needing_full_check.push(path.clone());
                },
                Ok(false) => {
                    // File unchanged (metadata match) - no action needed
                },
                Err(_) => {
                    // Error checking metadata, fallback to full check
                    files_needing_full_check.push(path.clone());
                }
            }
        }
    }
    
    // 3. Full check only for files with changed metadata
    for path in files_needing_full_check {
        let file_path = Path::new(&path);
        if file_path.exists() {
            // Compute actual file hash and compare
            let (current_file_id, _) = vos::chunk_and_save_file(file_path)?;
            let index_entry = index.entries.get(&path).unwrap();
            
            if current_file_id != index_entry.file_id {
                changes.push((path, FileStatus::Modified));
            }
            // If hashes match, file is actually unchanged despite metadata difference
        }
    }
    
    // 4. Check for untracked files
    let mut current_files = HashMap::new();
    scan_working_directory_fast(Path::new("."), "", &mut current_files)?;
    
    for (path, _) in &current_files {
        if !index.entries.contains_key(path) {
            changes.push((path.clone(), FileStatus::Untracked));
        }
    }
    
    // 5. Display results
    display_status_results(&changes)?;
    
    Ok(())
}

/// Reads the HEAD commit ID from .orb/refs/heads/main
#[allow(dead_code)]
fn read_head_commit_id() -> Result<ObjectId, Box<dyn std::error::Error>> {
    let head_ref_path = Path::new(".orb").join("refs").join("heads").join("main");
    
    if !head_ref_path.exists() {
        return Ok(String::new()); // Empty repository
    }
    
    let commit_id = fs::read_to_string(head_ref_path)?
        .trim()
        .to_string();
    
    Ok(commit_id)
}

/// Loads a commit object from the VOS store
#[allow(dead_code)]
fn load_commit_object(commit_id: &ObjectId) -> Result<Commit, Box<dyn std::error::Error>> {
    let object_data = load_object_data(commit_id)?;
    let commit: Commit = serde_json::from_slice(&object_data)?;
    Ok(commit)
}

/// Loads a directory object from the VOS store
#[allow(dead_code)]
fn load_directory_object(dir_id: &ObjectId) -> Result<Directory, Box<dyn std::error::Error>> {
    let object_data = load_object_data(dir_id)?;
    let directory: Directory = serde_json::from_slice(&object_data)?;
    Ok(directory)
}

/// Loads raw object data from the VOS store by ID
#[allow(dead_code)]
fn load_object_data(object_id: &ObjectId) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (prefix, suffix) = object_id.split_at(2);
    let object_path = Path::new(".orb")
        .join("objects")
        .join(prefix)
        .join(suffix);
    
    let data = fs::read(object_path)?;
    Ok(data)
}

/// Recursively builds a map of all tracked files and their object IDs
#[allow(dead_code)]
fn build_tracked_files_map(
    directory: &Directory,
    current_path: &str,
    tracked_files: &mut HashMap<String, ObjectId>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in &directory.entries {
        let entry_path = if current_path.is_empty() {
            entry.name.clone()
        } else {
            format!("{}/{}", current_path, entry.name)
        };
        
        if entry.mode == 0o040000 {
            // It's a directory, recurse into it
            let sub_directory = load_directory_object(&entry.id)?;
            build_tracked_files_map(&sub_directory, &entry_path, tracked_files)?;
        } else {
            // It's a file
            tracked_files.insert(entry_path, entry.id.clone());
        }
    }
    Ok(())
}

/// Recursively scans the working directory and computes file hashes
#[allow(dead_code)]
fn scan_working_directory(
    path: &Path,
    current_path: &str,
    current_files: &mut HashMap<String, ObjectId>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name = entry_path.file_name().unwrap().to_string_lossy().to_string();
        
        // Skip .orb directory
        if file_name == ".orb" {
            continue;
        }
        
        let full_path = if current_path.is_empty() {
            file_name.clone()
        } else {
            format!("{}/{}", current_path, file_name)
        };
        
        let metadata = fs::metadata(&entry_path)?;
        
        if metadata.is_dir() {
            // Recurse into subdirectory
            scan_working_directory(&entry_path, &full_path, current_files)?;
        } else if metadata.is_file() {
            // Compute file hash using our VOS chunking (for consistency)
            let (file_id, _) = vos::chunk_and_save_file(&entry_path)?;
            current_files.insert(full_path, file_id);
        }
    }
    Ok(())
}

/// Fast working directory scan - only collects paths, no hashing
fn scan_working_directory_fast(
    path: &Path,
    current_path: &str,
    current_files: &mut HashMap<String, bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name = entry_path.file_name().unwrap().to_string_lossy().to_string();
        
        // Skip .orb directory
        if file_name == ".orb" {
            continue;
        }
        
        let full_path = if current_path.is_empty() {
            file_name.clone()
        } else {
            format!("{}/{}", current_path, file_name)
        };
        
        let metadata = fs::metadata(&entry_path)?;
        
        if metadata.is_dir() {
            // Recurse into subdirectory
            scan_working_directory_fast(&entry_path, &full_path, current_files)?;
        } else if metadata.is_file() {
            // Just record the path exists
            current_files.insert(full_path, true);
        }
    }
    Ok(())
}

/// Displays the status results in a user-friendly format
fn display_status_results(changes: &[(String, FileStatus)]) -> Result<(), Box<dyn std::error::Error>> {
    if changes.is_empty() {
        println!("✅ Working directory is clean");
        println!("   Nothing to commit, working tree clean");
        return Ok(());
    }
    
    println!("## Changes in working directory:\n");
    
    // Group changes by status
    let modified: Vec<_> = changes.iter().filter(|(_, status)| *status == FileStatus::Modified).collect();
    let untracked: Vec<_> = changes.iter().filter(|(_, status)| *status == FileStatus::Untracked).collect();
    let deleted: Vec<_> = changes.iter().filter(|(_, status)| *status == FileStatus::Deleted).collect();
    
    if !modified.is_empty() {
        println!("📝 Modified files:");
        for (path, _) in modified {
            println!("   modified:   {}", path);
        }
        println!();
    }
    
    if !untracked.is_empty() {
        println!("❓ Untracked files:");
        for (path, _) in untracked {
            println!("   untracked:  {}", path);
        }
        println!();
    }
    
    if !deleted.is_empty() {
        println!("🗑️  Deleted files:");
        for (path, _) in deleted {
            println!("   deleted:    {}", path);
        }
        println!();
    }
    
    println!("To save these changes, use: orb save -m \"<commit message>\"");
    
    Ok(())
}