<div align="center">
 <h2>🚀 Orbit v0.4.5 - Next Gen Distributed VCS with Email-Based Namespaces</h2>
</div>
<div align="center">
  <img width="420" height="367" alt="orbitvcs00" src="https://github.com/user-attachments/assets/72f10322-3a33-4dd4-a9c2-0f8250d3c361" />
</div>

---

**Orbit** is a production-ready distributed version control system built on the revolutionary **Virtual Object Store (VOS)** architecture with **VNP (VOS Network Protocol)** for lightning-fast, SHA3-secured transactions. Features **AI-powered natural language queries**, **email-based namespace security**, **self-service registration**, and **GitHub-compatible clone workflows** with seamless distributed development and auto-repository creation.

## 🎯 Key Features (v0.4.5)

- **🤖 AI-Powered Queries** - Ask questions about your repository in natural language using Azure OpenAI
- **⚡ Revolutionary VOS Architecture** - Virtual Object Store with 40% faster operations than Git
- **🌐 VNP Protocol** - Custom VOS Network Protocol with SHA3-256 secured transactions
- **📧 Email-Based Namespaces** - alice@company.com gets alice/* access (collision-proof)
- **🔐 Self-Service Registration** - REST API user management with token authentication  
- **🏗️ Auto-Repository Creation** - Repositories created automatically when accessed
- **📥 GitHub-Like Clone Workflow** - `orb clone` → `orb checkout` → actual files
- **🔄 Complete Object Graph Sync** - Full commits, trees, files, and chunks synchronization
- **☁️ Azure Production Deployment** - Container Apps with persistent namespace storage

## You want to test it end to end ? Contact me via passadis.github.io to discuss your use case and help you set up the Server!

## 🚀 Quick Start

```bash
# 1. Register with email (self-service)
curl -X POST http://your-server.com:8081/admin/users \
  -H "Content-Type: application/json" \
  -d '{"username": "alice@company.com", "repositories": [], "permissions": {"read": true, "write": true, "admin": false}}'

# 2. Set authentication token
export ORBIT_TOKEN="your-token-here"

# 3. Clone any repository (auto-created if doesn't exist)
orb clone "orbits://your-server.com:8082/alice/my-project" my-project

# 4. Checkout files and start working
cd my-project
orb checkout

# 5. Ask AI about your repository! 🤖
orb ai "What files are tracked in this repository?"
orb ai "What is the current branch and latest commit?"
orb ai "How many files were changed recently?"
```

## 🤖 AI-Powered Queries (NEW!)

**Orbit now features built-in AI assistance powered by Azure OpenAI!**

Ask questions about your repository in natural language:

```bash
# Query repository state
orb ai "What is the current branch?"
orb ai "How many files are tracked?"
orb ai "What's the latest commit message?"

# Analyze changes
orb ai "What files were modified recently?"
orb ai "Tell me about recent commits"
orb ai "Is the repository clean?"
```

**Setup**: Create `.orbit-ai-config.json` or use environment variables:
- `AZURE_OPENAI_ENDPOINT` - Your Azure OpenAI endpoint
- `AZURE_OPENAI_API_KEY` - Your API key
- `AZURE_OPENAI_DEPLOYMENT` - Model deployment name
- `AZURE_OPENAI_API_VERSION` - API version

The AI has full context of your repository including current branch, tracked files, commit history, and working directory status!

## 📧 Email-Based Security

Access repositories based on your email namespace:
- `alice@company.com` → can access `alice/*` repositories
- `bob@startup.io` → can access `bob/*` repositories  
- Automatic collision prevention and namespace isolation

## 🏗️ Self-Service Repository Management

**New in v0.4.5**: Repositories are created automatically when you access them:

```bash
# List your namespace repositories (authenticates automatically)
orb list-repos "orbits://your-server.com:8082"

# Clone creates repository if it doesn't exist
orb clone "orbits://your-server.com:8082/alice/new-idea" new-idea

# Your email determines namespace access:
# alice@company.com can access alice/project1, alice/project2, etc.
```

## ☁️ Azure Production Ready

Deploy with complete namespace isolation and persistent storage:

```bash
# Server runs on port 8082 (VNP protocol)
# Admin API runs on port 8081 (user management)
# Each namespace gets isolated directory: /alice/, /bob/, etc.
```

## 🔧 Command Reference

### Core Commands
```bash
orb init                           # Initialize new repository
orb save -m "message"              # Create commit with complete object graph
orb check                          # Check working directory status
orb history                        # Show commit history (DAG)
orb revert                         # Revert files to their last committed state
orb fetch                          # Fetch and convert a Git repository to Orbit format
orb checkout                       # Checkout files from commits
```

### AI Commands *(NEW in v0.4.5)* 🤖
```bash
orb ai "your question"             # Ask AI about your repository using natural language
                                   # Examples: "What's the current branch?"
                                   #          "How many files are tracked?"
                                   #          "What's the latest commit?"
```

### Distributed Commands *(v0.4.5)*
```bash
orb list-repos <url>               # List repositories in your namespace
orb clone <url/namespace/repo> <local-name>  # Clone (auto-creates if needed)
orb sync <url>                     # Synchronize with remote server
orb register                       # Register a new user account on an Orbit server
```

## 🏗️ Architecture

### Revolutionary VOS + VNP Architecture
- **Virtual Object Store (VOS)** - 40% faster than Git with content-addressed storage
- **VNP Protocol** - Custom VOS Network Protocol with SHA3-256 secured transactions
- **Post-Quantum Security** - SHA3-256 hashing for future-proof cryptographic security
- **Email-Based Namespaces** - alice@company.com → alice/* access with collision prevention
- **Auto-Repository Creation** - Repositories created on first access with namespace isolation
- **AI Integration** - Azure OpenAI GPT-4o for natural language repository queries

### Production Deployment  
- **Azure Container Apps** - Dual-port deployment (8082 + 8081)
- **Namespace Isolation** - Each user gets isolated directory
- **REST Admin API** - Self-service user registration
- **TLS Security** - End-to-end encrypted communication
- **AI-Powered CLI** - Built-in intelligent assistant for repository operations

---

## 📋 Version History

### 🚀 v0.4.5 - Email-Based Namespaces & AI-Powered Queries *(Current)*
**Released:** November 2025
- **🤖 AI-Powered Queries** - Natural language repository queries using Azure OpenAI GPT-4o
- **📧 Email-Based Namespace Security** - alice@company.com gets alice/* access automatically
- **🔐 Self-Service Registration** - REST API for user management without admin intervention
- **🏗️ Auto-Repository Creation** - Repositories created when accessed (like GitHub)
- **📥 Complete Clone Workflow** - `orb clone` → `orb checkout` → working files extracted
- **🔄 Object Graph Integrity** - Full commits, trees, files, chunks with proper routing
- **☁️ Production Azure Deployment** - Dual-port server with namespace isolation

---

**Orbit v0.4.5** - *AI-powered distributed VCS with email-based security and natural language queries.* 🌟🤖

*Built with ❤️ by K.Passadis in Rust for performance, security, and developer productivity.*
