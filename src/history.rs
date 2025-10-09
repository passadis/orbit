use std::fs;
use std::path::Path;
use crate::objects::{ObjectId, Commit, Directory, File};

/// Displays the commit history by traversing the DAG backward from HEAD
pub fn show_history() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📚 Orbit History (orb history)\n");
    
    // 1. Get the current HEAD commit ID
    let head_commit_id = read_head_commit_id()?;
    
    if head_commit_id.is_empty() {
        println!("📝 No commits found (empty repository)");
        println!("\nTo create your first commit, use: orb save -m \"Initial commit\"");
        return Ok(());
    }

    // 2. Traverse the DAG backward from HEAD
    let mut current_commit_id = head_commit_id;
    let mut commit_count = 0;
    
    loop {
        // Load the current commit
        let commit = load_commit_object(&current_commit_id)?;
        commit_count += 1;
        
        // Display commit information
        let _short_id = &current_commit_id[0..7.min(current_commit_id.len())]; // TODO: Use for compact display
        let timestamp = format_timestamp(commit.timestamp);
        
        println!("commit {} (#{}) 📝", current_commit_id, commit_count);
        println!("Author: {}", commit.author);
        println!("Date:   {}", timestamp);
        println!("");
        println!("    {}", commit.message);
        println!("");
        
        // Move to parent commit
        if commit.parents.is_empty() {
            // Reached the root commit
            break;
        } else if commit.parents.len() == 1 {
            // Normal linear history
            current_commit_id = commit.parents[0].clone();
        } else {
            // Merge commit (for future versions)
            println!("    (Merge commit with {} parents)", commit.parents.len());
            current_commit_id = commit.parents[0].clone(); // Follow first parent
        }
    }
    
    println!("📊 Total commits: {}", commit_count);
    Ok(())
}

/// Reverts files to their state in the HEAD commit
pub fn revert_files(file_paths: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Orbit Revert (orb revert)\n");
    
    // 1. Get the current HEAD commit ID
    let head_commit_id = read_head_commit_id()?;
    
    if head_commit_id.is_empty() {
        println!("❌ No commits found - nothing to revert to");
        return Ok(());
    }

    // 2. Load the HEAD commit and its root directory
    let head_commit = load_commit_object(&head_commit_id)?;
    let root_directory = load_directory_object(&head_commit.tree)?;
    
    // 3. Build a map of all files in the commit
    let mut committed_files = std::collections::HashMap::new();
    build_file_map(&root_directory, "", &mut committed_files)?;
    
    // 4. Determine which files to revert
    let files_to_revert: Vec<String> = if file_paths.is_empty() {
        // Revert all files
        committed_files.keys().cloned().collect()
    } else {
        // Revert only specified files
        file_paths.into_iter()
            .filter(|path| committed_files.contains_key(path))
            .collect()
    };
    
    if files_to_revert.is_empty() {
        println!("⚠️  No files to revert (specified files not found in commit)");
        return Ok(());
    }
    
    // 5. Revert each file
    let mut reverted_count = 0;
    for file_path in files_to_revert {
        match revert_single_file(&file_path, &committed_files) {
            Ok(_) => {
                println!("✅ Reverted: {}", file_path);
                reverted_count += 1;
            },
            Err(e) => {
                println!("❌ Failed to revert {}: {}", file_path, e);
            }
        }
    }
    
    println!("\n🎉 Successfully reverted {} file(s) to HEAD commit", reverted_count);
    Ok(())
}

/// Reverts a single file to its committed state
fn revert_single_file(
    file_path: &str,
    committed_files: &std::collections::HashMap<String, ObjectId>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_id = committed_files.get(file_path)
        .ok_or("File not found in commit")?;
    
    // Load the File object
    let file_object = load_file_object(file_id)?;
    
    // Load and reassemble the file content from chunks
    let file_content = reassemble_file_content(&file_object)?;
    
    // Create directory if needed
    let file_path_obj = Path::new(file_path);
    if let Some(parent) = file_path_obj.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Write the content back to the working directory
    fs::write(file_path_obj, file_content)?;
    
    Ok(())
}

/// Loads a File object from the VOS store
fn load_file_object(file_id: &ObjectId) -> Result<File, Box<dyn std::error::Error>> {
    let object_data = load_object_data(file_id)?;
    let file_object: File = serde_json::from_slice(&object_data)?;
    Ok(file_object)
}

/// Reassembles file content from its chunks (simplified for MVP)
fn reassemble_file_content(file_object: &File) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // For MVP v0.2, we'll use a simplified approach
    // In the current implementation, the root_chunk_id represents the entire file
    // In a full implementation, this would traverse the Merkle tree of chunks
    
    let chunk_data = load_object_data(&file_object.root_chunk_id)?;
    Ok(chunk_data)
}

/// Builds a map of all files in a directory tree
fn build_file_map(
    directory: &Directory,
    current_path: &str,
    file_map: &mut std::collections::HashMap<String, ObjectId>,
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
            build_file_map(&sub_directory, &entry_path, file_map)?;
        } else {
            // It's a file
            file_map.insert(entry_path, entry.id.clone());
        }
    }
    Ok(())
}

/// Helper functions (reused from status.rs)
fn read_head_commit_id() -> Result<ObjectId, Box<dyn std::error::Error>> {
    let head_ref_path = Path::new(".orb").join("refs").join("heads").join("main");
    
    if !head_ref_path.exists() {
        return Ok(String::new());
    }
    
    let commit_id = fs::read_to_string(head_ref_path)?
        .trim()
        .to_string();
    
    Ok(commit_id)
}

fn load_commit_object(commit_id: &ObjectId) -> Result<Commit, Box<dyn std::error::Error>> {
    let object_data = load_object_data(commit_id)?;
    let commit: Commit = serde_json::from_slice(&object_data)?;
    Ok(commit)
}

fn load_directory_object(dir_id: &ObjectId) -> Result<Directory, Box<dyn std::error::Error>> {
    let object_data = load_object_data(dir_id)?;
    let directory: Directory = serde_json::from_slice(&object_data)?;
    Ok(directory)
}

fn load_object_data(object_id: &ObjectId) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (prefix, suffix) = object_id.split_at(2);
    let object_path = Path::new(".orb")
        .join("objects")
        .join(prefix)
        .join(suffix);
    
    let data = fs::read(object_path)?;
    Ok(data)
}

fn format_timestamp(timestamp: i64) -> String {
    use std::time::{UNIX_EPOCH, Duration};
    
    let system_time = UNIX_EPOCH + Duration::from_secs(timestamp as u64);
    
    // Simple formatting for MVP (in production, use chrono crate)
    format!("{:?}", system_time)
}