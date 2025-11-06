use clap::{Parser, Subcommand};
use tokio;
mod repo;
mod objects;
mod vos;
mod status;
mod index;
mod history;
mod fetch;
mod vnp;
mod client_tls;

// The main application structure for the 'orb' executable
#[derive(Parser, Debug)]
#[command(
    author = "Orbit Development Team", 
    version = "0.4.5", 
    about = "The next-generation version control system: ORBIT.", 
    long_about = "Orbit is a performance-focused, post-quantum secure version control system built on a Virtual Object Store (VOS) architecture. It delivers lightning-fast status checks and superior performance for incremental changes using SHA3-256 cryptographic hashing and content-defined chunking. Now featuring VNP (VOS Network Protocol) for distributed synchronization with complete object graph support and multi-repository architecture."
)]
struct OrbCli {
    #[command(subcommand)]
    command: Commands,
}

// Defines all the main subcommands (orb <command>)
#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a new Orbit repository in the current directory
    ///
    /// Creates a new .orb directory with the Virtual Object Store (VOS) structure,
    /// initializes the main branch, and sets up the repository metadata.
    Init, 
    
    /// Save changes to the repository, creating a new commit
    ///
    /// Processes all changes in the working directory, creates content-defined chunks
    /// using FastCDC, computes SHA3-256 hashes, and stores the new commit in the DAG.
    Save {
        /// The commit message describing the changes
        #[arg(short, long, help = "Commit message describing the changes")]
        message: String,
    },
    
    /// Check the status of the working directory
    ///
    /// Uses VOS Index optimization to quickly compare file metadata against the
    /// last commit, showing modified, added, and deleted files.
    #[command(alias = "status")]
    Check, 
    
    /// Show the commit history with DAG visualization
    ///
    /// Displays the directed acyclic graph (DAG) of commits showing relationships,
    /// commit messages, timestamps, and SHA3-256 hashes.
    History,
    
    /// Revert files to their last committed state
    ///
    /// Restores files from the VOS to match their state in the HEAD commit.
    /// If no files specified, reverts all modified files.
    Revert {
        /// Specific files to revert (if none specified, reverts all modified files)
        #[arg(help = "Files to revert to HEAD state")]
        files: Vec<String>,
    },
    
    /// Fetch and convert a Git repository to Orbit format
    ///
    /// Downloads a Git repository from a URL and converts it to Orbit's VOS format
    /// with post-quantum SHA3-256 hashing and content-defined chunking.
    Fetch {
        /// Git repository URL to fetch and convert
        #[arg(help = "Git repository URL (e.g., https://github.com/user/repo.git)")]
        url: String,
        
        /// Target directory name (optional, defaults to repository name)
        #[arg(short, long, help = "Target directory name")]
        target: Option<String>,
    },
    
    /// Synchronize with remote Orbit repositories
    ///
    /// Connects to a remote Orbit server and synchronizes commits using the VOS Network Protocol (VNP).
    /// Features post-quantum secure communication and efficient delta synchronization.
    Sync {
        /// Remote server URL (e.g., orbit://example.com:8080 or 127.0.0.1:8080)
        #[arg(help = "Remote Orbit server URL")]
        url: String,
    },
    
    /// Checkout files from a specific commit to the working directory
    ///
    /// Extracts files from a commit's tree and restores them to the working directory.
    /// This allows you to switch between different commit states or restore files after sync.
    Checkout {
        /// Commit ID to checkout (if not specified, uses HEAD)
        #[arg(help = "Commit ID to checkout (defaults to HEAD)")]
        commit_id: Option<String>,
    },
    
    /// Clone a repository from a remote Orbit server
    ///
    /// Creates a new local repository by downloading from a remote server.
    /// Supports multi-repository servers with repository selection.
    Clone {
        /// Remote server URL with optional repository path (e.g., server.com:8080/repo-name)
        #[arg(help = "Remote server URL with optional repository path")]
        url: String,
        
        /// Local directory name (optional, defaults to repository name)
        #[arg(help = "Local directory name")]
        directory: Option<String>,
    },
    
    /// List available repositories on a remote server
    ///
    /// Connects to an Orbit server and displays all available repositories.
    ListRepos {
        /// Remote server URL (e.g., server.com:8080)
        #[arg(help = "Remote Orbit server URL")]
        url: String,
    },
    
    /// Register a new user account on an Orbit server
    ///
    /// Creates a new user account with email-based namespace security.
    /// Your email becomes your username and automatically grants access to your namespace.
    /// Example: alice@company.com gets access to alice@company.com/* repositories.
    Register {
        /// Your email address (becomes your username for namespace security)
        #[arg(long, help = "Email address (e.g., alice@company.com) - becomes your username")]
        email: String,
        
        /// Orbit server URL (e.g., orbit.privapulse.com:8082)
        #[arg(long, help = "Orbit server URL for registration")]
        server: String,
        
        /// Username (deprecated - email is now used as username for security)
        #[arg(long, help = "DEPRECATED: Email is now used as username for namespace security")]
        username: Option<String>,
    },
}

/// Implementation of the 'orb sync' command logic.
async fn run_sync(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Connecting to Orbit server: {}", url);
    
    // Parse the URL to determine TLS requirements
    let orbit_url = client_tls::OrbitUrl::parse(url)?;
    
    println!("🌐 Establishing {} VNP connection to {}:{}...", 
        if orbit_url.use_tls { "TLS-secured" } else { "PQC-secured" },
        orbit_url.host, 
        orbit_url.port
    );
    
    // Establish connection (TLS or plain TCP)
    if orbit_url.use_tls {
        // TLS connection (use insecure mode for testing with self-signed certificates)
        let tls_client = client_tls::ClientTls::new_insecure()?;
        let tls_stream = tls_client.connect(&orbit_url.host, orbit_url.port, &orbit_url.server_name).await?;
        let (mut reader, mut writer) = tokio::io::split(tls_stream);
        return run_sync_with_stream(&mut reader, &mut writer, orbit_url.repository.as_deref()).await;
    } else {
        // Plain TCP connection
        let addr = format!("{}:{}", orbit_url.host, orbit_url.port);
        let stream = tokio::net::TcpStream::connect(&addr).await?;
        let (mut reader, mut writer) = stream.into_split();
        return run_sync_with_stream(&mut reader, &mut writer, orbit_url.repository.as_deref()).await;
    }
}

/// Run sync with established stream (both TLS and plain TCP)
async fn run_sync_with_stream<R, W>(
    reader: &mut R,
    writer: &mut W,
    repository: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>>
where
    R: tokio::io::AsyncReadExt + Unpin,
    W: tokio::io::AsyncWriteExt + Unpin,
{
    // Phase 0: Authentication - MANDATORY first step
    println!("🔐 Authenticating with server...");
    
    // Try to read token from environment variable or saved token file
    let token = match std::env::var("ORBIT_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            // Try to read from saved token file in home directory
            if let Ok(home_dir) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
                let token_file = std::path::Path::new(&home_dir).join(".orb_token");
                match std::fs::read_to_string(&token_file) {
                    Ok(token) => {
                        println!("🔑 Using saved authentication token");
                        token.trim().to_string()
                    },
                    Err(_) => {
                        eprintln!("❌ No authentication token found.");
                        eprintln!("💡 Register for a new account: orb register --email your@email.com --server orbit.privapulse.com:8082");
                        eprintln!("💡 Or set existing token: export ORBIT_TOKEN=\"your-token-here\"");
                        return Err("Authentication token required".into());
                    }
                }
            } else {
                eprintln!("❌ Cannot find home directory for token storage");
                return Err("Authentication token required".into());
            }
        }
    };
    
    // Send authentication token
    vnp::send_command(writer, vnp::VnpCommand::Authenticate(token)).await?;
    
    // Wait for authentication result
    match vnp::recv_command(reader).await? {
        vnp::VnpCommand::AuthResult { success, message } => {
            if success {
                println!("✅ Authenticated successfully");
            } else {
                eprintln!("❌ Authentication failed: {}", message);
                return Err("Authentication failed".into());
            }
        }
        vnp::VnpCommand::Error(msg) => {
            eprintln!("❌ Server error during authentication: {}", msg);
            return Err("Authentication error".into());
        }
        _ => {
            eprintln!("❌ Unexpected response during authentication");
            return Err("Unexpected authentication response".into());
        }
    }
    
    // Phase 1.5: Repository Selection (if repository path provided in URL)
    if let Some(repo_name) = repository {
        println!("📂 Selecting repository: {}", repo_name);
        vnp::send_command(writer, vnp::VnpCommand::SelectRepository(repo_name.to_string())).await?;
        
        // Wait for repository selection result
        match vnp::recv_command(reader).await? {
            vnp::VnpCommand::RepositorySelected(selected_repo) => {
                println!("✅ Repository '{}' selected", selected_repo);
            }
            vnp::VnpCommand::Error(msg) => {
                eprintln!("❌ Repository selection failed: {}", msg);
                return Err("Repository selection failed".into());
            }
            _ => {
                eprintln!("❌ Unexpected response during repository selection");
                return Err("Unexpected repository selection response".into());
            }
        }
    }
    
    // Get local HEAD commit
    let local_commits = match repo::get_local_commits() {
        Ok(commits) => commits,
        Err(_) => {
            println!("📝 No local commits found, starting fresh sync...");
            Vec::new()
        }
    };
    
    // Phase 1: Download Phase - Tell server what we have and download missing commits
    println!("📋 Negotiating with server ({} local commits)...", local_commits.len());
    vnp::send_command(writer, vnp::VnpCommand::Have(local_commits.clone())).await?;
    
    // Wait for server response with commits we need to download
    let server_commits = match vnp::recv_command(reader).await? {
        vnp::VnpCommand::Want(missing_commits) => {
            if missing_commits.is_empty() {
                println!("📥 No new commits to download from server");
            } else {
                println!("📥 Downloading {} commits from server...", missing_commits.len());
                
                // Phase 1b: Pull missing objects from server
                for commit_id in &missing_commits {
                    println!("  📦 Requesting commit: {}", commit_id);
                    vnp::send_command(writer, vnp::VnpCommand::Get(commit_id.clone())).await?;
                    
                    // Receive object header
                    match vnp::recv_command(reader).await? {
                        vnp::VnpCommand::ObjectHeader { id, object_type, size } => {
                            println!("  📄 Receiving {} object ({} bytes)...", object_type, size);
                            
                            // Receive object data
                            let object_data = vnp::recv_object_data(reader, size).await?;
                            
                            // Store object in local VOS
                            match store_received_object(&id, &object_type, &object_data) {
                                Ok(_) => println!("  ✅ Stored {} successfully", id),
                                Err(e) => {
                                    println!("  ⚠️ Warning: Could not store {}: {}", id, e);
                                    // Continue with other objects rather than failing completely
                                }
                            }
                        }
                        vnp::VnpCommand::Error(msg) => {
                            return Err(format!("Failed to get object {}: {}", commit_id, msg).into());
                        }
                        _ => {
                            return Err(format!("Unexpected response for object {}", commit_id).into());
                        }
                    }
                }
                
                println!("✅ Downloaded {} commits successfully!", missing_commits.len());
                
                // Phase 1c: Download complete object graphs for each commit
                println!("📥 Downloading complete object graphs...");
                for commit_id in &missing_commits {
                    download_complete_object_graph(reader, writer, commit_id).await?;
                }
                println!("✅ Downloaded complete object graphs!");
                
                // Update HEAD to point to the latest commit
                repo::update_head_after_sync(&missing_commits)?;
            }
            
            // Return server commits for upload phase
            missing_commits
        },
        vnp::VnpCommand::Error(msg) => {
            return Err(format!("Server error: {}", msg).into());
        },
        _ => {
            return Err("Unexpected server response during negotiation".into());
        }
    };
    
    // Phase 2: Upload Phase - Send our local commits that server doesn't have  
    if !local_commits.is_empty() {
        // Find commits we have that server doesn't have
        let commits_to_upload: Vec<String> = local_commits.iter()
            .filter(|commit| !server_commits.contains(commit))
            .cloned()
            .collect();
            
        if !commits_to_upload.is_empty() {
            println!("📤 Uploading {} local commits to server...", commits_to_upload.len());
            
            // Collect ALL objects needed for these commits (commits, trees, files, chunks)
            let mut all_objects_to_upload = Vec::new();
            let mut object_queue = commits_to_upload.clone();
            
            println!("🔍 Discovering all objects referenced by commits...");
            
            while let Some(object_id) = object_queue.pop() {
                if all_objects_to_upload.contains(&object_id) {
                    continue; // Already processed
                }
                
                // Load the object to analyze its references
                match load_local_object(&object_id) {
                    Ok((object_type, object_data)) => {
                        all_objects_to_upload.push(object_id.clone());
                        
                        // Parse object to find references based on type
                        if object_type == "commit" {
                            if let Ok(commit_json) = serde_json::from_slice::<serde_json::Value>(&object_data) {
                                if let Some(tree_id) = commit_json.get("tree").and_then(|v| v.as_str()) {
                                    if !all_objects_to_upload.contains(&tree_id.to_string()) {
                                        object_queue.push(tree_id.to_string());
                                    }
                                }
                            }
                        } else if object_type == "tree" {
                            if let Ok(tree_json) = serde_json::from_slice::<serde_json::Value>(&object_data) {
                                if let Some(entries) = tree_json.get("entries").and_then(|v| v.as_array()) {
                                    for entry in entries {
                                        if let Some(file_id) = entry.get("id").and_then(|v| v.as_str()) {
                                            if !all_objects_to_upload.contains(&file_id.to_string()) {
                                                object_queue.push(file_id.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        } else if object_type == "file" {
                            if let Ok(file_json) = serde_json::from_slice::<serde_json::Value>(&object_data) {
                                if let Some(root_chunk_id) = file_json.get("root_chunk_id").and_then(|v| v.as_str()) {
                                    if !all_objects_to_upload.contains(&root_chunk_id.to_string()) {
                                        object_queue.push(root_chunk_id.to_string());
                                    }
                                }
                                if let Some(chunk_ids) = file_json.get("chunk_ids").and_then(|v| v.as_array()) {
                                    for chunk_id_val in chunk_ids {
                                        if let Some(chunk_id) = chunk_id_val.as_str() {
                                            if !all_objects_to_upload.contains(&chunk_id.to_string()) {
                                                object_queue.push(chunk_id.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(format!("Failed to load local object {}: {}", object_id, e).into());
                    }
                }
            }
            
            println!("📦 Discovered {} total objects to upload", all_objects_to_upload.len());
            
            // Tell server we want to push commits (server will request objects)
            vnp::send_command(writer, vnp::VnpCommand::Push(commits_to_upload.clone())).await?;
            
            // Server will request only the objects it actually needs
            // We need to handle a variable number of SendObject requests
            let mut uploaded_count = 0;
            loop {
                match vnp::recv_command(reader).await? {
                    vnp::VnpCommand::SendObject(requested_id) => {
                        // Verify this is one of the objects we can provide
                        if !all_objects_to_upload.contains(&requested_id) {
                            return Err(format!("Server requested unexpected object: {}", requested_id).into());
                        }
                        
                        println!("  📤 Uploading object: {}", requested_id);
                        
                        // Load object from local VOS
                        match load_local_object(&requested_id) {
                            Ok((object_type, object_data)) => {
                                // Send object header
                                vnp::send_command(writer, vnp::VnpCommand::ObjectHeader {
                                    id: requested_id.clone(),
                                    object_type: object_type.clone(),
                                    size: object_data.len(),
                                }).await?;
                                
                                // Send object data
                                vnp::send_object_data(writer, &object_data).await?;
                                println!("  ✅ Uploaded {} ({} bytes)", requested_id, object_data.len());
                                uploaded_count += 1;
                            }
                            Err(e) => {
                                return Err(format!("Failed to load local object {}: {}", requested_id, e).into());
                            }
                        }
                    }
                    vnp::VnpCommand::Ok => {
                        // Server confirms upload phase is complete
                        println!("✅ Uploaded {} commits successfully!", uploaded_count);
                        break;
                    }
                    vnp::VnpCommand::Error(msg) => {
                        return Err(format!("Server error during upload: {}", msg).into());
                    }
                    _ => {
                        return Err("Unexpected server response during upload phase".into());
                    }
                }
            }
            
            println!("✅ Uploaded {} commits successfully!", commits_to_upload.len());
        } else {
            println!("📤 No new local commits to upload");
        }
    }
    
    // Phase 2: Finalization
    vnp::send_command(writer, vnp::VnpCommand::Ready).await?;
    
    match vnp::recv_command(reader).await? {
        vnp::VnpCommand::Ok => {
            println!("✅ Synchronization completed successfully!");
        },
        vnp::VnpCommand::Error(msg) => {
            return Err(format!("Sync finalization error: {}", msg).into());
        },
        _ => {
            return Err("Unexpected server response during finalization".into());
        }
    }
    
    Ok(())
}

/// Stores a received object in the local VOS
fn store_received_object(id: &str, object_type: &str, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    match object_type {
        "commit" => {
            // Verify it's valid JSON commit data
            let _commit: objects::Commit = serde_json::from_slice(data)?;
            vos::store_object_with_id(id, data)?;
            println!("    📝 Stored commit object");
        }
        "tree" => {
            // Verify it's valid JSON tree data  
            let _tree: objects::Directory = serde_json::from_slice(data)?;
            vos::store_object_with_id(id, data)?;
            println!("    🌳 Stored tree object");
        }
        "file" => {
            // Verify it's valid JSON file data
            let _file: objects::File = serde_json::from_slice(data)?;
            vos::store_object_with_id(id, data)?;
            println!("    📄 Stored file object");
        }
        _ => {
            return Err(format!("Unknown object type: {}", object_type).into());
        }
    }
    Ok(())
}

/// Loads an object from the local VOS for uploading
fn load_local_object(id: &str) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>> {
    // Load object data from VOS using the same pattern as history module
    let (prefix, suffix) = id.split_at(2);
    let object_path = std::path::Path::new(".orb")
        .join("objects")
        .join(prefix)
        .join(suffix);
    
    let object_data = std::fs::read(object_path)?;
    
    // Determine object type by trying to parse as different types
    if let Ok(_commit) = serde_json::from_slice::<objects::Commit>(&object_data) {
        return Ok(("commit".to_string(), object_data));
    }
    
    if let Ok(_tree) = serde_json::from_slice::<objects::Directory>(&object_data) {
        return Ok(("tree".to_string(), object_data));
    }
    
    if let Ok(_file) = serde_json::from_slice::<objects::File>(&object_data) {
        return Ok(("file".to_string(), object_data));
    }
    
    // If it's not a structured object, it's likely a chunk (raw data)
    // Chunks are just raw bytes, not JSON
    if !object_data.is_empty() {
        return Ok(("chunk".to_string(), object_data));
    }
    
    Err(format!("Could not determine type of object: {}", id).into())
}

/// Checkout files from a specific commit to the working directory
fn checkout_commit(commit_id: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Orbit Checkout");
    
    // Determine which commit to checkout
    let target_commit = match commit_id {
        Some(id) => {
            println!("📍 Checking out commit: {}", id);
            id.to_string()
        }
        None => {
            // Use HEAD commit
            let head_path = std::path::Path::new(".orb").join("refs").join("heads").join("main");
            if !head_path.exists() {
                return Err("No HEAD commit found. Repository might be empty.".into());
            }
            let head_commit = std::fs::read_to_string(head_path)?.trim().to_string();
            println!("📍 Checking out HEAD commit: {}", head_commit);
            head_commit
        }
    };
    
    // Load the commit object
    let commit_data = load_object_from_vos(&target_commit)?;
    let commit: objects::Commit = serde_json::from_slice(&commit_data)?;
    
    println!("📋 Commit: {}", commit.message);
    println!("🌳 Restoring files from tree: {}", commit.tree);
    
    // Load and process the root tree
    restore_tree_to_working_dir(&commit.tree, "")?;
    
    println!("✅ Checkout completed successfully!");
    Ok(())
}

/// Recursively restore a tree and its contents to the working directory
fn restore_tree_to_working_dir(tree_id: &str, path_prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
    let tree_data = load_object_from_vos(tree_id)?;
    let directory: objects::Directory = serde_json::from_slice(&tree_data)?;
    
    for entry in &directory.entries {
        let full_path = if path_prefix.is_empty() {
            entry.name.clone()
        } else {
            format!("{}/{}", path_prefix, entry.name)
        };
        
        if entry.mode == 0o040000 {
            // Directory
            println!("  � Restoring directory: {}", full_path);
            std::fs::create_dir_all(&full_path)?;
            restore_tree_to_working_dir(&entry.id, &full_path)?;
        } else if entry.mode == 0o100644 {
            // Regular file
            println!("  � Restoring file: {}", full_path);
            restore_file_to_working_dir(&entry.id, &full_path)?;
        } else {
            println!("  ⚠️ Skipping unknown entry type: {} (mode: {:o})", full_path, entry.mode);
        }
    }
    
    Ok(())
}

/// Restore a single file from VOS to the working directory
fn restore_file_to_working_dir(file_id: &str, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Load the File object
    let file_data = load_object_from_vos(file_id)?;
    let file_object: objects::File = serde_json::from_slice(&file_data)?;
    
    // Load the actual file content from the root chunk
    let content_data = load_object_from_vos(&file_object.root_chunk_id)?;
    
    // Create parent directories if needed
    if let Some(parent) = std::path::Path::new(file_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Write the file content
    std::fs::write(file_path, content_data)?;
    
    Ok(())
}

/// Load an object from the VOS by ID
fn load_object_from_vos(object_id: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (prefix, suffix) = object_id.split_at(2);
    let object_path = std::path::Path::new(".orb")
        .join("objects")
        .join(prefix)
        .join(suffix);
    
    let data = std::fs::read(object_path)?;
    Ok(data)
}

/// Download complete object graph for a commit (trees, files, and chunks)
async fn download_complete_object_graph<R, W>(
    reader: &mut R,
    writer: &mut W, 
    commit_id: &str
) -> Result<(), Box<dyn std::error::Error>>
where
    R: tokio::io::AsyncReadExt + Unpin,
    W: tokio::io::AsyncWriteExt + Unpin,
{
    println!("  🌳 Downloading object graph for commit: {}", commit_id);
    
    // Load the commit object (should already be downloaded)
    let commit_data = load_object_from_vos(commit_id)?;
    let commit: objects::Commit = serde_json::from_slice(&commit_data)?;
    
    // Download the root tree recursively
    download_tree_recursive(reader, writer, &commit.tree).await?;
    
    Ok(())
}

/// Recursively download a tree and all its contents
async fn download_tree_recursive<R, W>(
    reader: &mut R,
    writer: &mut W,
    tree_id: &str
) -> Result<(), Box<dyn std::error::Error>>
where
    R: tokio::io::AsyncReadExt + Unpin,
    W: tokio::io::AsyncWriteExt + Unpin,
{
    // Check if we already have this tree
    if object_exists_locally(tree_id) {
        return Ok(()); // Skip if we already have it
    }
    
    println!("    📁 Downloading tree: {}", tree_id);
    
    // Request the tree object
    vnp::send_command(writer, vnp::VnpCommand::GetTree(tree_id.to_string())).await?;
    
    // Receive tree object
    match vnp::recv_command(reader).await? {
        vnp::VnpCommand::ObjectHeader { id, object_type, size } => {
            if object_type != "tree" {
                return Err(format!("Expected tree object, got {}", object_type).into());
            }
            
            // Receive tree data
            let tree_data = vnp::recv_object_data(reader, size).await?;
            
            // Store tree object
            store_received_object(&id, &object_type, &tree_data)?;
            
            // Parse tree to get its entries
            let directory: objects::Directory = serde_json::from_slice(&tree_data)?;
            
            // Recursively download all entries
            for entry in &directory.entries {
                if entry.mode == 0o040000 {
                    // Directory - recurse
                    Box::pin(download_tree_recursive(reader, writer, &entry.id)).await?;
                } else if entry.mode == 0o100644 {
                    // File - download file and its chunks
                    Box::pin(download_file_recursive(reader, writer, &entry.id)).await?;
                }
            }
        }
        vnp::VnpCommand::Error(msg) => {
            return Err(format!("Failed to get tree {}: {}", tree_id, msg).into());
        }
        _ => {
            return Err(format!("Unexpected response for tree {}", tree_id).into());
        }
    }
    
    Ok(())
}

/// Download a file object and its chunk data
async fn download_file_recursive<R, W>(
    reader: &mut R,
    writer: &mut W,
    file_id: &str
) -> Result<(), Box<dyn std::error::Error>>
where
    R: tokio::io::AsyncReadExt + Unpin,
    W: tokio::io::AsyncWriteExt + Unpin,
{
    // Check if we already have this file
    if object_exists_locally(file_id) {
        return Ok(()); // Skip if we already have it
    }
    
    println!("    📄 Downloading file: {}", file_id);
    
    // Request the file object
    vnp::send_command(writer, vnp::VnpCommand::GetFile(file_id.to_string())).await?;
    
    // Receive file object
    match vnp::recv_command(reader).await? {
        vnp::VnpCommand::ObjectHeader { id, object_type, size } => {
            if object_type != "file" {
                return Err(format!("Expected file object, got {}", object_type).into());
            }
            
            // Receive file data
            let file_data = vnp::recv_object_data(reader, size).await?;
            
            // Store file object
            store_received_object(&id, &object_type, &file_data)?;
            
            // Parse file to get its chunk ID
            let file_object: objects::File = serde_json::from_slice(&file_data)?;
            
            // Download the chunk data
            download_chunk(reader, writer, &file_object.root_chunk_id).await?;
        }
        vnp::VnpCommand::Error(msg) => {
            return Err(format!("Failed to get file {}: {}", file_id, msg).into());
        }
        _ => {
            return Err(format!("Unexpected response for file {}", file_id).into());
        }
    }
    
    Ok(())
}

/// Download a chunk (raw file content)
async fn download_chunk<R, W>(
    reader: &mut R,
    writer: &mut W,
    chunk_id: &str
) -> Result<(), Box<dyn std::error::Error>>
where
    R: tokio::io::AsyncReadExt + Unpin,
    W: tokio::io::AsyncWriteExt + Unpin,
{
    // Check if we already have this chunk
    if object_exists_locally(chunk_id) {
        return Ok(()); // Skip if we already have it
    }
    
    println!("      📦 Downloading chunk: {}", chunk_id);
    
    // Request the chunk object (using Get command since chunks are raw data)
    vnp::send_command(writer, vnp::VnpCommand::Get(chunk_id.to_string())).await?;
    
    // Receive chunk object
    match vnp::recv_command(reader).await? {
        vnp::VnpCommand::ObjectHeader { id, object_type: _, size } => {
            // Receive chunk data
            let chunk_data = vnp::recv_object_data(reader, size).await?;
            
            // Store chunk directly (chunks are raw data, not JSON)
            vos::store_object_with_id(&id, &chunk_data)?;
            println!("      ✅ Stored chunk {} ({} bytes)", id, chunk_data.len());
        }
        vnp::VnpCommand::Error(msg) => {
            return Err(format!("Failed to get chunk {}: {}", chunk_id, msg).into());
        }
        _ => {
            return Err(format!("Unexpected response for chunk {}", chunk_id).into());
        }
    }
    
    Ok(())
}

/// Check if an object exists locally in VOS
fn object_exists_locally(object_id: &str) -> bool {
    let (prefix, suffix) = object_id.split_at(2);
    let object_path = std::path::Path::new(".orb")
        .join("objects")
        .join(prefix)
        .join(suffix);
    
    object_path.exists()
}



/// Validate email format
fn is_valid_email(email: &str) -> bool {
    email.contains('@') && 
    email.len() > 3 &&
    !email.starts_with('@') &&
    !email.ends_with('@') &&
    email.chars().all(|c| c.is_alphanumeric() || "@.-_".contains(c))
}

/// Register a new user on an Orbit server
async fn register_user(email: &str, server: &str, username: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 Registering new user account...");
    println!("📧 Email: {}", email);
    println!("🌐 Server: {}", server);
    
    // Validate email format
    if !is_valid_email(email) {
        return Err("Invalid email format. Please provide a valid email address (e.g., alice@company.com)".into());
    }
    
    // Use email as username for namespace security (ignore optional username parameter)
    let username = email;
    let namespace = email.split('@').next().unwrap_or("user");
    
    println!("👤 Username: {} (email-based for security)", username);
    println!("🏷️  Namespace: {} (auto-granted access to {}//*)", namespace, namespace);
    
    // Parse server URL to get admin API endpoint
    let orbit_url = client_tls::OrbitUrl::parse(server)?;
    let admin_api_url = format!("http://{}:8081/admin/users", orbit_url.host);
    
    println!("🔗 Connecting to Admin API: {}", admin_api_url);
    
    // Create user registration request - server will auto-grant namespace access
    let registration_request = serde_json::json!({
        "username": username, // Email format for namespace security
        "repositories": [], // Server auto-grants namespace access (alice@company.com/*)
        "permissions": {
            "read": true,
            "write": true, 
            "admin": false
        }
    });
    
    // Send registration request to Admin API
    let client = reqwest::Client::new();
    let response = client
        .post(&admin_api_url)
        .json(&registration_request)
        .send()
        .await?;
    
    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;
        
        if let Some(token) = result.get("token").and_then(|t| t.as_str()) {
            println!("🎉 Registration successful!");
            println!("🔑 Your authentication token: {}", token);
            println!("");
            println!("💡 To use your token:");
            println!("   export ORBIT_TOKEN=\"{}\"", token);
            println!("");
            println!("🚀 You can now create repositories:");
            println!("   orb push orbits://{}:{}/{}/my-project", orbit_url.host, orbit_url.port, username);
            
            // Save token to user's home directory
            if let Ok(home_dir) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
                let token_file = std::path::Path::new(&home_dir).join(".orb_token");
                if let Ok(()) = std::fs::write(&token_file, token) {
                    println!("💾 Token saved to: {}", token_file.display());
                    println!("💡 Token will be automatically loaded in future sessions");
                }
            }
        } else {
            return Err("Registration succeeded but no token received".into());
        }
    } else {
        let error_text = response.text().await?;
        return Err(format!("Registration failed: {}", error_text).into());
    }
    
    Ok(())
}

/// List available repositories on a remote server
async fn list_repositories(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Listing repositories on server: {}", url);
    
    // Parse the URL to determine TLS requirements
    let orbit_url = client_tls::OrbitUrl::parse(url)?;
    
    println!("🌐 Connecting to {}:{}...", orbit_url.host, orbit_url.port);
    
    // Establish connection
    if orbit_url.use_tls {
        let tls_client = client_tls::ClientTls::new_insecure()?;
        let tls_stream = tls_client.connect(&orbit_url.host, orbit_url.port, &orbit_url.server_name).await?;
        let (mut reader, mut writer) = tokio::io::split(tls_stream);
        list_repositories_impl(&mut reader, &mut writer).await
    } else {
        let stream = tokio::net::TcpStream::connect(format!("{}:{}", orbit_url.host, orbit_url.port)).await?;
        let (mut reader, mut writer) = stream.into_split();
        list_repositories_impl(&mut reader, &mut writer).await
    }
}

/// Implementation of repository listing
async fn list_repositories_impl<R, W>(
    reader: &mut R,
    writer: &mut W,
) -> Result<(), Box<dyn std::error::Error>>
where
    R: tokio::io::AsyncReadExt + Unpin,
    W: tokio::io::AsyncWriteExt + Unpin,
{
    // Authenticate first - load token from environment or file
    let token = match std::env::var("ORBIT_TOKEN") {
        Ok(token) => {
            println!("🔑 Using environment token");
            token
        }
        Err(_) => {
            // Try to read from saved token file in home directory
            if let Ok(home_dir) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
                let token_file = std::path::Path::new(&home_dir).join(".orb_token");
                match std::fs::read_to_string(&token_file) {
                    Ok(token) => {
                        println!("🔑 Using saved authentication token");
                        token.trim().to_string()
                    },
                    Err(_) => {
                        eprintln!("❌ No authentication token found.");
                        eprintln!("💡 Register for a new account: orb register --email your@email.com --server orbit.privapulse.com:8082");
                        eprintln!("💡 Or set existing token: export ORBIT_TOKEN=\"your-token-here\"");
                        return Err("Authentication token required".into());
                    }
                }
            } else {
                eprintln!("❌ Cannot find home directory for token storage");
                return Err("Authentication token required".into());
            }
        }
    };
    
    println!("🔐 Authenticating with server...");
    vnp::send_command(writer, vnp::VnpCommand::Authenticate(token)).await?;
    
    // Wait for authentication result
    match vnp::recv_command(reader).await? {
        vnp::VnpCommand::AuthResult { success, message } => {
            if success {
                println!("✅ Authenticated successfully");
            } else {
                eprintln!("❌ Authentication failed: {}", message);
                return Err("Authentication failed".into());
            }
        }
        vnp::VnpCommand::Error(msg) => {
            eprintln!("❌ Server error during authentication: {}", msg);
            return Err("Authentication error".into());
        }
        _ => {
            return Err("Unexpected authentication response".into());
        }
    }
    
    // Send list repositories command
    vnp::send_command(writer, vnp::VnpCommand::ListRepositories).await?;
    
    // Receive repository list
    match vnp::recv_command(reader).await? {
        vnp::VnpCommand::RepositoryList(repos) => {
            if repos.is_empty() {
                println!("📂 No repositories found on server");
            } else {
                println!("📂 Available repositories ({}):", repos.len());
                for (i, repo) in repos.iter().enumerate() {
                    println!("  {}. {}", i + 1, repo);
                }
            }
            Ok(())
        }
        vnp::VnpCommand::Error(msg) => {
            Err(format!("Server error: {}", msg).into())
        }
        _ => {
            Err("Unexpected response from server".into())
        }
    }
}

/// Clone a repository from a remote server
async fn clone_repository(url: &str, directory: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("📥 Cloning repository from: {}", url);
    
    // Parse the full URL to extract repository information
    let orbit_url = client_tls::OrbitUrl::parse(url)?;
    let repo_name = orbit_url.repository.as_deref();
    
    // Determine local directory name
    let local_dir = match directory {
        Some(dir) => dir.to_string(),
        None => repo_name.unwrap_or("orbit-repo").to_string(),
    };
    
    // Create local directory and initialize
    std::fs::create_dir_all(&local_dir)?;
    std::env::set_current_dir(&local_dir)?;
    
    // Initialize Orbit repository
    repo::init()?;
    println!("✅ Initialized local repository in: {}", local_dir);
    
    // Connect and sync
    println!("🌐 Connecting to {}:{}...", orbit_url.host, orbit_url.port);
    
    if orbit_url.use_tls {
        let tls_client = client_tls::ClientTls::new_insecure()?;
        let tls_stream = tls_client.connect(&orbit_url.host, orbit_url.port, &orbit_url.server_name).await?;
        let (mut reader, mut writer) = tokio::io::split(tls_stream);
        clone_repository_impl(&mut reader, &mut writer, repo_name).await
    } else {
        let stream = tokio::net::TcpStream::connect(format!("{}:{}", orbit_url.host, orbit_url.port)).await?;
        let (mut reader, mut writer) = stream.into_split();
        clone_repository_impl(&mut reader, &mut writer, repo_name).await
    }
}

/// Implementation of repository cloning
async fn clone_repository_impl<R, W>(
    reader: &mut R,
    writer: &mut W,
    repo_name: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>>
where
    R: tokio::io::AsyncReadExt + Unpin,
    W: tokio::io::AsyncWriteExt + Unpin,
{
    // Authenticate first - load token from environment or file
    let token = match std::env::var("ORBIT_TOKEN") {
        Ok(token) => {
            println!("🔑 Using environment token");
            token
        }
        Err(_) => {
            // Try to read from saved token file in home directory
            if let Ok(home_dir) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
                let token_file = std::path::Path::new(&home_dir).join(".orb_token");
                match std::fs::read_to_string(&token_file) {
                    Ok(token) => {
                        println!("🔑 Using saved authentication token");
                        token.trim().to_string()
                    },
                    Err(_) => {
                        eprintln!("❌ No authentication token found.");
                        eprintln!("💡 Register for a new account: orb register --email your@email.com --server orbit.privapulse.com:8082");
                        eprintln!("💡 Or set existing token: export ORBIT_TOKEN=\"your-token-here\"");
                        return Err("Authentication token required".into());
                    }
                }
            } else {
                eprintln!("❌ Cannot find home directory for token storage");
                return Err("Authentication token required".into());
            }
        }
    };
    
    println!("🔐 Authenticating with server...");
    vnp::send_command(writer, vnp::VnpCommand::Authenticate(token)).await?;
    
    // Wait for authentication result
    match vnp::recv_command(reader).await? {
        vnp::VnpCommand::AuthResult { success, message } => {
            if success {
                println!("✅ Authenticated successfully");
            } else {
                eprintln!("❌ Authentication failed: {}", message);
                return Err("Authentication failed".into());
            }
        }
        vnp::VnpCommand::Error(msg) => {
            eprintln!("❌ Server error during authentication: {}", msg);
            return Err("Authentication error".into());
        }
        _ => {
            return Err("Unexpected authentication response".into());
        }
    }
    
    // If specific repository requested, select it first
    if let Some(repo) = repo_name {
        println!("📂 Selecting repository: {}", repo);
        vnp::send_command(writer, vnp::VnpCommand::SelectRepository(repo.to_string())).await?;
        
        match vnp::recv_command(reader).await? {
            vnp::VnpCommand::RepositorySelected(selected) => {
                println!("✅ Selected repository: {}", selected);
            }
            vnp::VnpCommand::Error(msg) => {
                // If repository doesn't exist, try to create it
                if msg.contains("not found") {
                    println!("📂 Repository '{}' not found, creating it...", repo);
                    vnp::send_command(writer, vnp::VnpCommand::CreateRepository(repo.to_string())).await?;
                    
                    match vnp::recv_command(reader).await? {
                        vnp::VnpCommand::RepositorySelected(created) => {
                            println!("✅ Created and selected repository: {}", created);
                        }
                        vnp::VnpCommand::Error(create_msg) => {
                            return Err(format!("Failed to create repository '{}': {}", repo, create_msg).into());
                        }
                        _ => {
                            return Err("Unexpected response to repository creation".into());
                        }
                    }
                } else {
                    return Err(format!("Failed to select repository '{}': {}", repo, msg).into());
                }
            }
            _ => {
                return Err("Unexpected response to repository selection".into());
            }
        }
    }
    
    // Now perform standard sync to download all commits
    println!("📥 Downloading repository content...");
    
    // Use the same sync logic as run_sync but with existing reader/writer
    let local_commits = repo::get_local_commits().unwrap_or_default();
    println!("📋 Negotiating with server ({} local commits)...", local_commits.len());

    // Send our commit list to server (HAVE)
    vnp::send_command(writer, vnp::VnpCommand::Have(local_commits.clone())).await?;

    // Receive server's response (WANT)  
    let missing_commits = match vnp::recv_command(reader).await? {
        vnp::VnpCommand::Want(commits) => commits,
        vnp::VnpCommand::Error(msg) => return Err(format!("Server error: {}", msg).into()),
        _ => return Err("Expected WANT response from server".into()),
    };

    if missing_commits.is_empty() {
        println!("✅ Already up to date!");
        return Ok(());
    }

    println!("📥 Downloading {} commits from server...", missing_commits.len());

    // Download missing commits
    for commit_id in &missing_commits {
        println!("  📦 Requesting commit: {}", commit_id);
        vnp::send_command(writer, vnp::VnpCommand::Get(commit_id.clone())).await?;

        match vnp::recv_command(reader).await? {
            vnp::VnpCommand::ObjectHeader { id, object_type, size } => {
                println!("  📄 Receiving {} object ({} bytes)...", object_type, size);
                let object_data = vnp::recv_object_data(reader, size).await?;
                store_received_object(&id, &object_type, &object_data)?;
                println!("  ✅ Stored {} successfully", id);
            }
            vnp::VnpCommand::Error(msg) => {
                return Err(format!("Failed to get commit {}: {}", commit_id, msg).into());
            }
            _ => {
                return Err(format!("Unexpected response for commit {}", commit_id).into());
            }
        }
    }

    println!("✅ Downloaded {} commits successfully!", missing_commits.len());

    // Download complete object graphs
    println!("📥 Downloading complete object graphs...");
    for commit_id in &missing_commits {
        download_complete_object_graph(reader, writer, commit_id).await?;
    }

    // Signal completion
    vnp::send_command(writer, vnp::VnpCommand::Ready).await?;

    // Wait for server confirmation
    match vnp::recv_command(reader).await? {
        vnp::VnpCommand::Ok => {
            println!("✅ Sync completed successfully!");
        }
        vnp::VnpCommand::Error(msg) => {
            return Err(format!("Sync failed: {}", msg).into());
        }
        _ => {
            return Err("Unexpected response from server".into());
        }
    }
    
    // Update HEAD to point to the latest commit after cloning
    if !missing_commits.is_empty() {
        repo::update_head_after_sync(&missing_commits)?;
        println!("📍 Updated HEAD to: {}", missing_commits.last().unwrap());
    }
    
    println!("✅ Repository cloned successfully!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = OrbCli::parse();

    match &cli.command {
        Commands::Init => {
            if let Err(e) = repo::init() {
                eprintln!("❌ Initialization failed: {}", e);
            }
        },
        Commands::Save { message } => {
            match repo::save_snapshot(message) {
                Ok(_) => {}, // Success message is printed in repo::save_snapshot
                Err(e) => eprintln!("❌ Save failed: {}", e),
            }
        },
        Commands::Check => {
            if let Err(e) = status::check_status() {
                eprintln!("❌ Status check failed: {}", e);
            }
        },
        Commands::History => {
            if let Err(e) = history::show_history() {
                eprintln!("❌ History display failed: {}", e);
            }
        },
        Commands::Revert { files } => {
            if let Err(e) = history::revert_files(files.clone()) {
                eprintln!("❌ Revert failed: {}", e);
            }
        },
        Commands::Fetch { url, target } => {
            if let Err(e) = fetch::fetch_git_repository(url, target.as_deref()) {
                eprintln!("❌ Fetch failed: {}", e);
            }
        },
        Commands::Sync { url } => {
            match run_sync(url).await {
                Ok(_) => {},
                Err(e) => eprintln!("❌ Sync failed: {}", e),
            }
        },
        Commands::Checkout { commit_id } => {
            if let Err(e) = checkout_commit(commit_id.as_deref()) {
                eprintln!("❌ Checkout failed: {}", e);
            }
        }
        Commands::Clone { url, directory } => {
            match clone_repository(url, directory.as_deref()).await {
                Ok(()) => println!("✅ Repository cloned successfully!"),
                Err(e) => eprintln!("❌ Clone failed: {}", e),
            }
        }
        Commands::ListRepos { url } => {
            match list_repositories(url).await {
                Ok(()) => println!("✅ Repository list retrieved!"),
                Err(e) => eprintln!("❌ Failed to list repositories: {}", e),
            }
        }
        Commands::Register { email, server, username } => {
            match register_user(email, server, username.as_deref()).await {
                Ok(()) => println!("✅ User registration successful!"),
                Err(e) => eprintln!("❌ Registration failed: {}", e),
            }
        }
    }
    
    Ok(())
}