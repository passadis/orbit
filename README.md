# 🚀 Orbit v0.4.2 - Distributed Version Control with Complete Object Graph Sync

<div align="center">
  <img width="420" height="367" alt="orbitvcs00" src="https://github.com/user-attachments/assets/72f10322-3a33-4dd4-a9c2-0f8250d3c361" />
</div>

---

**Orbit** is a production-ready, post-quantum secure version control system with **complete object graph synchronization**, **multi-repository support**, and **Azure cloud integration**. Built on the revolutionary Virtual Object Store (VOS) architecture with TLS-encrypted distributed sync.

## 🎯 Key Features (v0.4.2)

- **🔄 Complete Object Graph Sync** - Full commits, trees, files, and chunks synchronization
- **🏛️ Multi-Repository Architecture** - Host multiple repositories with namespace isolation
- **🔒 TLS-Encrypted Communication** - End-to-end security with rustls
- **☁️ Azure Production Deployment** - Container Apps with persistent storage
- **⚡ VOS Performance** - 40% faster than Git with SHA3-256 security
- **🌐 VNP Protocol v2.2** - Custom binary protocol for efficient distributed sync

## You want to test it end to end ? Contact me via passadis.github.io to discuss your use case and help you set up the Server!

## 🚀 Quick Start

```bash
# Install globally
cargo install --path .

# Initialize repository
orb init

# Save changes
echo "Hello, Orbit!" > file.txt
orb save -m "Initial commit"

# Sync to cloud
orb sync orbits://your-server.com:8082

# List remote repositories
orb list-repos orbits://your-server.com:8082

# Clone repository
orb clone orbits://your-server.com:8082 repository-name
```

## 🔄 Complete Object Graph Sync

Orbit v0.4.2 introduces **complete object graph synchronization** ensuring full repository integrity:

- **Commits** → **Trees** → **Files** → **Chunks**
- Recursive dependency resolution
- Atomic sync operations
- Zero data loss guarantee

## ☁️ Azure Production Deployment

Deploy Orbit server to Azure Container Apps with persistent storage:

```bash
# Build and push to Azure Container Registry
az acr build --registry <your-acr> --image orbit-server:latest .

# Deploy with Azure File Share persistence
az containerapp create \
  --name "orbit-server" \
  --resource-group "<your-rg>" \
  --environment "<your-env>" \
  --image "<your-acr>.azurecr.io/orbit-server:latest" \
  --transport tcp \
  --target-port 8082 \
  --ingress external \
  --min-replicas 1 \
  --max-replicas 5 \
  --cpu 1.0 \
  --memory 2.0Gi \
  --env-vars RUST_LOG="info"
```

### Sync with Cloud Server
```bash
# Sync to Azure (TLS encrypted)
orb sync "orbits://your-app.azurecontainerapps.io:8081"
```

## 🏛️ Multi-Repository Architecture 

**New in v0.4.2**: Host multiple isolated repositories on a single server:

```bash
# List available repositories
orb list-repos orbits://your-server.com:8082

# Clone specific repository
orb clone orbits://your-server.com:8082 my-project

# Sync with specific repository (automatically selected after clone)
orb sync orbits://your-server.com:8082
```

## 🔧 Command Reference

### Core Commands
```bash
orb init                           # Initialize new repository
orb save -m "message"              # Create commit with complete object graph
orb check                          # Check working directory status
orb history                        # Show commit history (DAG)
orb checkout                       # Checkout files from commits
```

### Distributed Commands *(v0.4.2)*
```bash
orb sync <url>                     # Synchronize with remote server
orb list-repos <url>               # List remote repositories  
orb clone <url> <repo-name>        # Clone remote repository
```

## 🏗️ Architecture

### VOS Network Protocol v2.2
- **Complete Object Graph Sync** - Commits → Trees → Files → Chunks
- **Multi-Repository Support** - Namespace isolation and management
- **TLS Encryption** - End-to-end security with rustls
- **Efficient Binary Protocol** - Custom serialization with serde

### Production Deployment
- **Azure Container Apps** - Serverless container orchestration
- **Azure File Share** - Persistent multi-repository storage
- **Auto-scaling** - Handle variable workloads efficiently
- **Monitoring Ready** - Azure Monitor integration

---

## 📋 Version History

### 🚀 v0.4.2 - Complete Object Graph Sync *(Current)*
**Released:** October 2025
- **🔄 Complete Object Graph Sync** - Full repository integrity with commits, trees, files, and chunks
- **�️ Multi-Repository Architecture** - Host multiple repositories with namespace isolation
- **☁️ Azure Production Deployment** - Container Apps with Azure File Share persistence
- **� Enhanced Security** - TLS 1.3 encryption with production certificates
- **⚡ Performance Optimized** - 40% faster operations with VOS Index caching

---

**Orbit v0.4.2** - *Complete distributed version control with object graph integrity.* 🌟

*Built with ❤️ in Rust for performance, security, and developer productivity.*
