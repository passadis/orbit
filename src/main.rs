use clap::{Parser, Subcommand};
mod repo;
mod objects;
mod vos;
mod status;
mod index;
mod history;
mod fetch;

// The main application structure for the 'orb' executable
#[derive(Parser, Debug)]
#[command(
    author = "Orbit Development Team", 
    version = "0.3.0", 
    about = "The next-generation version control system: ORBIT.", 
    long_about = "Orbit is a performance-focused, post-quantum secure version control system built on a Virtual Object Store (VOS) architecture. It delivers lightning-fast status checks and superior performance for incremental changes using SHA3-256 cryptographic hashing and content-defined chunking."
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
    
    /// Synchronize with remote repositories (v0.4+ feature)
    ///
    /// Future feature for distributed version control with remote synchronization.
    /// Currently not implemented - use 'fetch' for Git repository conversion.
    Sync,
}

fn main() {
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
        Commands::Sync => {
            eprintln!("🚧 'orb sync' is not available in v0.3 (local-only). Use 'orb fetch' for Git repository conversion.");
        }
    }
}