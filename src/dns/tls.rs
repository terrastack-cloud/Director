use rustls::crypto::aws_lc_rs::default_provider;
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use rustls::ServerConfig;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, RwLock};
use tracing;

use crate::config::TlsCertConfig;

#[derive(Debug)]
pub struct DynamicCertResolver {
    pub tls_cert_config: TlsCertConfig,
    cache: RwLock<HashMap<String, Arc<CertifiedKey>>>,
}

impl DynamicCertResolver {
    pub fn new(tls_cert_config: TlsCertConfig) -> Self {
        DynamicCertResolver {
            tls_cert_config,
            cache: RwLock::new(HashMap::new()),
        }
    }
}

impl ResolvesServerCert for DynamicCertResolver {
    fn resolve(&self, hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        let domain = hello.server_name()?.to_string();

        // Check cache first
        if let Some(cached_key) = self.cache.read().unwrap().get(&domain) {
            tracing::debug!("Serving certificate for {} from cache", domain);
            return Some(Arc::clone(cached_key));
        }

        // --- Load Certificate Chain ---
        let cert_file = match File::open(&self.tls_cert_config.cert_path) {
            Ok(f) => f,
            Err(e) => {
                tracing::error!(
                    "Failed to open cert file {}: {}",
                    self.tls_cert_config.cert_path,
                    e
                );
                return None;
            }
        };
        let mut reader = BufReader::new(cert_file);
        let certs = match rustls_pemfile::certs(&mut reader).collect::<Result<Vec<_>, _>>() {
            Ok(certs) => certs,
            Err(e) => {
                tracing::error!(
                    "Failed to parse cert file {}: {}",
                    self.tls_cert_config.cert_path,
                    e
                );
                return None;
            }
        };

        // --- Load Private Key ---
        let key_file = match File::open(&self.tls_cert_config.key_path) {
            Ok(f) => f,
            Err(e) => {
                tracing::error!(
                    "Failed to open key file {}: {}",
                    self.tls_cert_config.key_path,
                    e
                );
                return None;
            }
        };
        let mut reader = BufReader::new(key_file);
        let key = match rustls_pemfile::private_key(&mut reader) {
            Ok(Some(key)) => key,
            Ok(None) => {
                tracing::error!("No private key found in {}", self.tls_cert_config.key_path);
                return None;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to parse key file {}: {}",
                    self.tls_cert_config.key_path,
                    e
                );
                return None;
            }
        };

        // --- Create Signing Key and CertifiedKey ---
        let signing_key = match rustls::crypto::aws_lc_rs::sign::any_supported_type(&key) {
            Ok(key) => key,
            Err(e) => {
                tracing::error!("Failed to create signing key: {}", e);
                return None;
            }
        };

        let ck = Arc::new(CertifiedKey::new(certs, signing_key));

        self.cache
            .write()
            .expect("RwLock should not be poisoned")
            .insert(domain.clone(), Arc::clone(&ck));
        tracing::debug!("Loaded and cached certificate for {}", domain);

        Some(ck)
    }
}
pub fn tls_server_config(
    protocol: &[u8],
    server_cert_resolver: Arc<dyn ResolvesServerCert>,
) -> eyre::Result<ServerConfig, eyre::Error> {
    let mut config = ServerConfig::builder_with_provider(Arc::new(default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|e| eyre::eyre!("error creating TLS acceptor: {e}"))?
        .with_no_client_auth()
        .with_cert_resolver(server_cert_resolver);

    config.alpn_protocols = vec![protocol.to_vec()];

    Ok(config)
}
