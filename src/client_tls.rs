use tokio_rustls::{TlsConnector, rustls::{ClientConfig, RootCertStore}};
use rustls_pki_types::ServerName;
use webpki_roots;
use std::sync::Arc;

/// TLS client configuration for secure VNP connections
pub struct ClientTls {
    connector: TlsConnector,
}

impl ClientTls {
    /// Create a new TLS client with system root certificates
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        
        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
            
        let connector = TlsConnector::from(Arc::new(config));
        
        Ok(ClientTls { connector })
    }
    
    /// Create a TLS client that accepts self-signed certificates (INSECURE - for testing only)
    pub fn new_insecure() -> Result<Self, Box<dyn std::error::Error>> {
        use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
        use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
        use rustls::{DigitallySignedStruct, SignatureScheme};
        
        #[derive(Debug)]
        struct InsecureVerifier;
        
        impl ServerCertVerifier for InsecureVerifier {
            fn verify_server_cert(
                &self,
                _end_entity: &CertificateDer<'_>,
                _intermediates: &[CertificateDer<'_>],
                _server_name: &ServerName<'_>,
                _ocsp_response: &[u8],
                _now: UnixTime,
            ) -> Result<ServerCertVerified, rustls::Error> {
                Ok(ServerCertVerified::assertion())
            }
            
            fn verify_tls12_signature(
                &self,
                _message: &[u8],
                _cert: &CertificateDer<'_>,
                _dss: &DigitallySignedStruct,
            ) -> Result<HandshakeSignatureValid, rustls::Error> {
                Ok(HandshakeSignatureValid::assertion())
            }
            
            fn verify_tls13_signature(
                &self,
                _message: &[u8],
                _cert: &CertificateDer<'_>,
                _dss: &DigitallySignedStruct,
            ) -> Result<HandshakeSignatureValid, rustls::Error> {
                Ok(HandshakeSignatureValid::assertion())
            }
            
            fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
                vec![
                    SignatureScheme::RSA_PKCS1_SHA1,
                    SignatureScheme::ECDSA_SHA1_Legacy,
                    SignatureScheme::RSA_PKCS1_SHA256,
                    SignatureScheme::ECDSA_NISTP256_SHA256,
                    SignatureScheme::RSA_PKCS1_SHA384,
                    SignatureScheme::ECDSA_NISTP384_SHA384,
                    SignatureScheme::RSA_PKCS1_SHA512,
                    SignatureScheme::ECDSA_NISTP521_SHA512,
                    SignatureScheme::RSA_PSS_SHA256,
                    SignatureScheme::RSA_PSS_SHA384,
                    SignatureScheme::RSA_PSS_SHA512,
                    SignatureScheme::ED25519,
                    SignatureScheme::ED448,
                ]
            }
        }
        
        let config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(InsecureVerifier))
            .with_no_client_auth();
            
        let connector = TlsConnector::from(Arc::new(config));
        
        Ok(ClientTls { connector })
    }
    
    /// Connect to a TLS-enabled server
    pub async fn connect(&self, host: &str, port: u16, server_name: &str) -> Result<tokio_rustls::client::TlsStream<tokio::net::TcpStream>, Box<dyn std::error::Error>> {
        // Create TCP connection
        let addr = format!("{}:{}", host, port);
        let stream = tokio::net::TcpStream::connect(&addr).await?;
        
        // Perform TLS handshake
        let domain = ServerName::try_from(server_name.to_string())?;
        let tls_stream = self.connector.connect(domain, stream).await?;
        
        Ok(tls_stream)
    }
}

/// Detect if a URL requires TLS
pub fn requires_tls(url: &str) -> bool {
    url.starts_with("https://") || 
    url.starts_with("orbits://") ||  // Secure Orbit protocol
    url.contains(":443") ||          // Standard HTTPS port
    url.contains(":8443")           // Standard secure alternate port
}

/// Parse Orbit URL and extract connection details
pub struct OrbitUrl {
    pub host: String,
    pub port: u16,
    pub use_tls: bool,
    pub server_name: String,
    pub repository: Option<String>,
}

impl OrbitUrl {
    pub fn parse(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let use_tls = requires_tls(url);
        
        // Remove protocol prefixes
        let clean_url = url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("orbits://")  // Secure Orbit
            .trim_start_matches("orbit://");   // Plain Orbit
        
        // Parse host and port (handle repository paths like host:port/repo/path)
        let (host, port) = if let Some(colon_pos) = clean_url.find(':') {
            let host = clean_url[..colon_pos].to_string();
            let remainder = &clean_url[colon_pos + 1..];
            
            // Find the port (everything before the first slash or end of string)
            let port_str = if let Some(slash_pos) = remainder.find('/') {
                &remainder[..slash_pos]
            } else {
                remainder
            };
            
            let port = port_str.parse::<u16>()?;
            (host, port)
        } else {
            // Default ports
            let host = if let Some(slash_pos) = clean_url.find('/') {
                clean_url[..slash_pos].to_string()
            } else {
                clean_url.to_string()
            };
            let port = if use_tls { 443 } else { 8080 };
            (host, port)
        };
        
        let server_name = host.clone();
        
        // Extract repository path if present
        let repository = if let Some(slash_pos) = clean_url.find('/') {
            // Find the part after host:port/
            let after_host_port = if clean_url.contains(':') {
                if let Some(port_start) = clean_url.find(':') {
                    let after_port = &clean_url[port_start + 1..];
                    if let Some(repo_start) = after_port.find('/') {
                        Some(after_port[repo_start + 1..].to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                // No port, repository starts after first slash
                Some(clean_url[slash_pos + 1..].to_string())
            };
            after_host_port
        } else {
            None
        };
        
        Ok(OrbitUrl {
            host,
            port,
            use_tls,
            server_name,
            repository,
        })
    }
}