use serde::{Serialize, Deserialize};
use crate::objects::ObjectId;
use std::io;

// --- VNP Command Types ---

/// Commands exchanged between the Orbit client and server.
#[derive(Debug, Serialize, Deserialize)]
pub enum VnpCommand {
    /// Client: Announces the commit IDs it possesses.
    Have(Vec<ObjectId>), 

    /// Server: Responds with the commit IDs the client must fetch.
    Want(Vec<ObjectId>), 

    /// Client: Requests a specific VOS object (Commit, Tree, or File).
    Get(ObjectId), 

    /// Server: Sends the object header (ID, type, and size).
    ObjectHeader {
        id: ObjectId,
        object_type: String, // "commit", "tree", or "file"
        size: usize,
    },

    /// Server: Sends raw binary object data following ObjectHeader.
    ObjectData(Vec<u8>), 
    
    /// Client: Push commits to server (sends list of commit IDs to upload)
    Push(Vec<ObjectId>),
    
    /// Client: Pull commits from server (requests download of specific commits)  
    Pull(Vec<ObjectId>),
    
    /// Server: Requests client to send a specific object
    SendObject(ObjectId),
    
    /// Client: Requests complete object graph for a commit (recursively gets trees, files, chunks)
    GetCompleteGraph(ObjectId),
    
    /// Client: Requests a tree object specifically
    GetTree(ObjectId),
    
    /// Client: Requests a file object specifically  
    GetFile(ObjectId),
    
    /// Multi-repository support commands (v2.2)
    /// Client: Request list of available repositories
    ListRepositories,
    /// Client: Select repository to work with
    SelectRepository(String),
    /// Client: Create new repository
    CreateRepository(String),
    /// Server: Respond with available repositories
    RepositoryList(Vec<String>),
    /// Server: Confirm repository selection
    RepositorySelected(String),
    
    /// Status command used by either side to signal phase transition.
    Ready, 

    /// Server: Signals successful operation (e.g., ref update confirmed).
    Ok, 

    /// Server: Signals an error (e.g., object not found, bad hash).
    Error(String), 
}

// --- VNP Network Utilities (Async Senders/Receivers) ---

/// Sends a VnpCommand over an asynchronous stream.
pub async fn send_command<W: tokio::io::AsyncWriteExt + Unpin>(
    writer: &mut W,
    command: VnpCommand,
) -> io::Result<()> {
    // Serialize the command into a JSON string (for MVP simplicity)
    let json_str = serde_json::to_string(&command).unwrap();
    
    // Send the length of the command, followed by the command itself, 
    // ensuring the receiver knows when the command ends.
    writer.write_u32(json_str.len() as u32).await?;
    writer.write_all(json_str.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

/// Receives a VnpCommand from an asynchronous stream.
pub async fn recv_command<R: tokio::io::AsyncReadExt + Unpin>(
    reader: &mut R,
) -> io::Result<VnpCommand> {
    // Read the command length
    let len = reader.read_u32().await? as usize;
    
    // Read the command data
    let mut buffer = vec![0u8; len];
    reader.read_exact(&mut buffer).await?;

    // Deserialize the JSON command
    let command: VnpCommand = serde_json::from_slice(&buffer)?;
    Ok(command)
}

/// Sends raw object data in chunks over the stream
pub async fn send_object_data<W: tokio::io::AsyncWriteExt + Unpin>(
    writer: &mut W,
    data: &[u8],
) -> io::Result<()> {
    const CHUNK_SIZE: usize = 8192; // 8KB chunks for efficient streaming
    
    for chunk in data.chunks(CHUNK_SIZE) {
        send_command(writer, VnpCommand::ObjectData(chunk.to_vec())).await?;
    }
    Ok(())
}

/// Receives object data by collecting ObjectData chunks until complete
pub async fn recv_object_data<R: tokio::io::AsyncReadExt + Unpin>(
    reader: &mut R,
    expected_size: usize,
) -> io::Result<Vec<u8>> {
    let mut received_data = Vec::with_capacity(expected_size);
    
    while received_data.len() < expected_size {
        match recv_command(reader).await? {
            VnpCommand::ObjectData(chunk) => {
                received_data.extend_from_slice(&chunk);
            }
            VnpCommand::Error(msg) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Server error: {}", msg)));
            }
            _ => {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Expected ObjectData"));
            }
        }
    }
    
    Ok(received_data)
}