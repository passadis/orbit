use serde::{Serialize, Deserialize};

// --- Type Aliases for Clarity and Security ---

// Object ID: A 32-byte SHA-3 (Keccak-256) hash, represented as a hex string.
pub type ObjectId = String; 

// --- Core VOS Objects ---

/// 1. The Chunk (Blob) Object
/// In VOS, the raw file content is broken into Chunks.
/// We don't need a specific struct for the *content* itself, 
/// as it's just raw bytes stored by its ID (hash).

/// 2. The File (Merkle Tree Root) Object
/// This object replaces Git's 'Blob' for files and holds the Merkle root
/// hash, proving the integrity and sequence of all data chunks.
#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    // The ID of the root chunk (or Merkle root) that points to all file data.
    // This is the true ID of the file's content.
    pub root_chunk_id: ObjectId, 
    pub size: usize,
}

/// 3. The Directory (Tree) Object
/// This object is equivalent to Git's 'Tree' and represents a folder snapshot.
#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub mode: u32,             // File permissions/type (e.g., file, directory)
    pub name: String,
    pub id: ObjectId,          // ID of the File or nested Directory object
}

/// A container for all entries in a directory.
#[derive(Debug, Serialize, Deserialize)]
pub struct Directory {
    pub entries: Vec<DirectoryEntry>,
}

/// 4. The Commit (DAG Node) Object
/// This object is the node in our Directed Acyclic Graph.
#[derive(Debug, Serialize, Deserialize)]
pub struct Commit {
    // ID of the root Directory object for this snapshot.
    pub tree: ObjectId, 
    // Parent commits (0 for initial, 1 for normal, 2+ for merge).
    pub parents: Vec<ObjectId>, 
    pub author: String,
    pub timestamp: i64,
    pub message: String,
    // PQC Signature (Placeholder for full implementation in later versions)
    pub signature: Option<String>, 
}