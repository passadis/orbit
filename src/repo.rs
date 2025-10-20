use std::fs;
use std::path::Path;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::vos;
use crate::objects::{self, ObjectId};
use crate::index::VosIndex;
// use rayon::prelude::*; // TODO: Enable for parallel processing in future versions

const ORB_DIR: &str = ".orb";

pub fn init() -> Result<(), std::io::Error> {
    let root = Path::new(ORB_DIR);

    if root.exists() {
        // We'll return an error or a message if the repository already exists
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Orbit repository already initialized.",
        ));
    }

    // 1. Create the .orb directory
    fs::create_dir(root)?;
    
    // 2. Create subdirectories
    fs::create_dir(root.join("objects"))?;
    fs::create_dir(root.join("refs"))?;
    
    // 3. Initialize config file (using a simple text format for MVP)
    let mut config_file = fs::File::create(root.join("config"))?;
    config_file.write_all(b"[core]\n")?;
    config_file.write_all(b"version = 0.1\n")?;
    config_file.write_all(b"hash_algorithm = sha3-256\n")?;
    
    // 4. Set the initial HEAD reference (default branch)
    let mut head_file = fs::File::create(root.join("HEAD"))?;
    head_file.write_all(b"ref: refs/heads/main\n")?;

    println!("✅ Initialized empty Orbit repository in {}", root.display());
    Ok(())
}
/// Recursively traverses the directory, chunks files, saves VOS objects, and builds the Directory (Tree).
/// Also updates the VOS Index with file metadata for fast status checks.
fn traverse_and_save_tree(path: &Path, current_path: &str, index: &mut VosIndex) -> Result<ObjectId, std::io::Error> {
    let mut entries = Vec::new();
    let iter = fs::read_dir(path)?;

    for entry in iter {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name = entry_path.file_name().unwrap().to_string_lossy().to_string();

        // Skip internal .orb directory
        if file_name == ORB_DIR {
            continue;
        }

        let full_path = if current_path.is_empty() {
            file_name.clone()
        } else {
            format!("{}/{}", current_path, file_name)
        };

        let metadata = fs::metadata(&entry_path)?;

        let (mode, id) = if metadata.is_dir() {
            // Recursive call for subdirectories
            let dir_id = traverse_and_save_tree(&entry_path, &full_path, index)?;
            (0o040000, dir_id) // Directory mode
        } else if metadata.is_file() {
            // Process file using Content-Defined Chunking and PQC hashing
            let (file_id, _file_size) = vos::chunk_and_save_file(&entry_path)?;
            
            // Update VOS Index with file metadata
            let (mtime, size) = VosIndex::get_file_metadata(&entry_path).unwrap_or((0, 0));
            index.update_entry(full_path.clone(), mtime, size, file_id.clone());
            
            (0o100644, file_id) // Regular file mode
        } else {
            // Skip other types (symlinks, etc., for MVP)
            continue;
        };

        entries.push(objects::DirectoryEntry {
            mode,
            name: file_name,
            id,
        });
    }

    // 1. Create and hash the Directory object
    let directory_obj = objects::Directory { entries };
    let dir_id = vos::hash_object(&directory_obj).unwrap();
    
    // 2. Save the Directory object metadata
    vos::save_object(&serde_json::to_vec(&directory_obj).unwrap());

    Ok(dir_id)
}

/// Orchestrates the entire 'orb save' process.
pub fn save_snapshot(message: &str) -> Result<(), std::io::Error> {
    // 1. Get current HEAD (parent commit)
    let parent_id = get_head_commit_id()?;

    // 2. Initialize or load the VOS Index
    let mut index = VosIndex::load().unwrap_or_else(|_| VosIndex::new());
    
    // Clear the index for fresh rebuild (ensures accuracy)
    index.clear();

    // 3. Build the new root Directory (Tree) and update VOS Index
    let root_dir_id = traverse_and_save_tree(Path::new("."), "", &mut index)?;

    // 4. Save the updated VOS Index
    if let Err(e) = index.save() {
        eprintln!("Warning: Could not save VOS Index: {}", e);
    }

    // 5. Create the Commit object
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    let commit_obj = objects::Commit {
        tree: root_dir_id,
        parents: if parent_id.is_empty() { vec![] } else { vec![parent_id.clone()] },
        author: "Orb Developer <dev@orbit.vcs>".to_string(), // TODO: Replace with user config
        timestamp: now,
        message: message.to_string(),
        signature: None, 
    };

    // 6. Hash and save the Commit object
    let commit_id = vos::hash_object(&commit_obj).unwrap();
    vos::save_object(&serde_json::to_vec(&commit_obj).unwrap());

    // 7. Update the main branch reference (HEAD)
    update_head(&commit_id)?;

    println!("✨ Saved commit {} to main: {}", &commit_id[0..7], message);
    Ok(())
}

// --- Helper Functions (Stubs for MVP) ---

/// Reads the current commit ID pointed to by HEAD. (Placeholder for now)
fn get_head_commit_id() -> Result<ObjectId, std::io::Error> {
    // In v0.1, we assume no parent for the first commit, or read the last commit's hash.
    // For a simple MVP, let's return an empty string, signifying a root commit.
    // In a future version, this reads the hash from .orb/refs/heads/main
    Ok("".to_string())
}

/// Updates the main branch ref to point to the new commit ID.
fn update_head(commit_id: &ObjectId) -> Result<(), std::io::Error> {
    // In v0.1, we'll write the commit ID directly to the main ref file
    let ref_path = Path::new(ORB_DIR).join("refs").join("heads").join("main");
    fs::create_dir_all(ref_path.parent().unwrap())?; // Ensure refs/heads exists
    let mut ref_file = fs::File::create(ref_path)?;
    ref_file.write_all(commit_id.as_bytes())?;
    Ok(())
}

/// Gets local commit IDs for synchronization with remote repositories
pub fn get_local_commits() -> Result<Vec<ObjectId>, std::io::Error> {
    let mut commits = Vec::new();
    
    // Read the HEAD commit (main branch)
    let head_path = Path::new(ORB_DIR).join("refs").join("heads").join("main");
    if head_path.exists() {
        let head_content = fs::read_to_string(head_path)?;
        let head_commit = head_content.trim().to_string();
        if !head_commit.is_empty() {
            commits.push(head_commit);
        }
    }
    
    // TODO: In future versions, traverse the commit DAG to get all commits
    // For v0.3.3 MVP, we'll just return the HEAD commit
    
    Ok(commits)
}

/// Updates HEAD to point to the latest synchronized commit
pub fn update_head_after_sync(commit_ids: &[ObjectId]) -> Result<(), std::io::Error> {
    if !commit_ids.is_empty() {
        // For now, use the last commit as HEAD (in future versions, we'll determine the proper HEAD)
        let latest_commit = &commit_ids[commit_ids.len() - 1];
        update_head(latest_commit)?;
        println!("📍 Updated HEAD to: {}", latest_commit);
    }
    Ok(())
}