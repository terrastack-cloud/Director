use thiserror::Error;

#[derive(Debug, Error)]
pub enum DnsError {
    #[error("Failed to parse listen address: {0}")]
    ParseListenAddress(String),
    #[error("Failed to create Tokio runtime: {0}")]
    TokioRuntimeCreation(#[from] std::io::Error),
    #[error("Failed to bind UDP on {0}: {1}")]
    UdpSocketBind(String, std::io::Error),
    #[error("Failed to bind TCP on {0}: {1}")]
    TcpSocketBind(String, std::io::Error),
    #[error("DNS server error: {0}")]
    DnsServer(String),
}
