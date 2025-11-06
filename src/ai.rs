use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// Configuration for Azure OpenAI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureOpenAiConfig {
    pub endpoint: String,
    pub api_key: String,
    pub deployment_name: String,
    pub api_version: String,
}

impl AzureOpenAiConfig {
    /// Load configuration from multiple sources with priority:
    /// 1. .env file (via dotenv)
    /// 2. Environment variables
    /// 3. .orbit-ai-config.json file
    /// 4. Hardcoded defaults
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        // Try to load .env file (silently fail if not present)
        let _ = dotenv::dotenv();

        // Try environment variables first
        if let (Ok(endpoint), Ok(api_key)) = (
            std::env::var("AZURE_OPENAI_ENDPOINT"),
            std::env::var("AZURE_OPENAI_API_KEY"),
        ) {
            let deployment_name = std::env::var("AZURE_OPENAI_DEPLOYMENT")
                .unwrap_or_else(|_| "gpt-4o-2024-11-20".to_string());
            let api_version = std::env::var("AZURE_OPENAI_API_VERSION")
                .unwrap_or_else(|_| "2024-08-01-preview".to_string());

            return Ok(Self {
                endpoint,
                api_key,
                deployment_name,
                api_version,
            });
        }

        // Try loading from config file
        if let Ok(config) = Self::from_config_file() {
            return Ok(config);
        }

        // Fall back to hardcoded defaults (for development)
        Ok(Self {
            endpoint: "https://develop-oai-az.openai.azure.com".to_string(),
            api_key: std::env::var("AZURE_OPENAI_API_KEY")
                .unwrap_or_else(|_| "your-api-key-here".to_string()),
            deployment_name: "gpt-4o-2024-11-20".to_string(),
            api_version: "2024-08-01-preview".to_string(),
        })
    }

    /// Load configuration from .orbit-ai-config.json file
    pub fn from_config_file() -> Result<Self, Box<dyn Error>> {
        // Try current directory first
        let local_path = PathBuf::from(".orbit-ai-config.json");
        if local_path.exists() {
            let content = fs::read_to_string(&local_path)?;
            return Ok(serde_json::from_str(&content)?);
        }

        // Try home directory
        if let Some(home) = dirs::home_dir() {
            let home_path = home.join(".orbit-ai-config.json");
            if home_path.exists() {
                let content = fs::read_to_string(&home_path)?;
                return Ok(serde_json::from_str(&content)?);
            }
        }

        Err("Configuration file not found".into())
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// Repository context for AI queries
#[derive(Debug, Serialize)]
pub struct RepoContext {
    pub current_branch: String,
    pub files: Vec<String>,
    pub recent_commits: Vec<CommitInfo>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
}

/// Azure OpenAI chat message
#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// Azure OpenAI API request
#[derive(Debug, Serialize)]
struct ChatRequest {
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

/// Azure OpenAI API response
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

/// Main AI interface
pub struct OrbitAI {
    config: AzureOpenAiConfig,
    client: reqwest::Client,
}

impl OrbitAI {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let config = AzureOpenAiConfig::from_env()?;
        let client = reqwest::Client::new();
        Ok(Self { config, client })
    }

    pub async fn query(&self, user_message: &str, context: &RepoContext) -> Result<String, Box<dyn Error>> {
        // Build the system prompt with repository context
        let system_prompt = format!(
            "You are an AI assistant for Orbit VCS, a modern version control system. \
             You help users understand their repository state and answer questions about their code.\n\n\
             Current Repository Context:\n\
             - Branch: {}\n\
             - Files: {} files tracked\n\
             - Status: {}\n\
             - Recent Commits: {}\n\n\
             File list: {}\n\n\
             Answer the user's question based on this context. Be concise and helpful.",
            context.current_branch,
            context.files.len(),
            context.status,
            context.recent_commits.len(),
            context.files.join(", ")
        );

        let request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
            max_tokens: 800,
            temperature: 0.7,
        };

        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.config.endpoint, self.config.deployment_name, self.config.api_version
        );

        let response = self
            .client
            .post(&url)
            .header("api-key", &self.config.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Azure OpenAI API error ({}): {}", status, error_text).into());
        }

        let chat_response: ChatResponse = response.json().await?;

        if let Some(choice) = chat_response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err("No response from AI".into())
        }
    }
}
