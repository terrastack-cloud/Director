use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer as RustlsCert, PrivateKeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use std::collections::HashMap;
use std::path::Path;
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

        if let Some(cached_key) = self.cache.read().unwrap().get(&domain) {
            tracing::debug!("Serving certificate for {} from cache", domain);
            return Some(Arc::clone(cached_key));
        }

        let cert_path = Path::new(&self.tls_cert_config.cert_path);
        let key_path = Path::new(&self.tls_cert_config.key_path);

        let cert = match RustlsCert::from_pem_file(cert_path) {
            Ok(c) => vec![c],
            Err(e) => {
                tracing::error!("Failed to parse certificate from {}: {}", cert_path.display(), e);
                return None;
            }
        };

        let key = match PrivateKeyDer::from_pem_file(key_path) {
            Ok(k) => k,
            Err(e) => {
                tracing::error!("Failed to get key from {}: {}", key_path.display(), e);
                return None;
            }
        };

        let signature_alg = match rustls::crypto::ring::sign::any_supported_type(&key) {
            Ok(alg) => alg,
            Err(e) => {
                tracing::error!("Failed to get key type for {}: {}", key_path.display(), e);
                return None;
            }
        };

        let ck = Arc::new(CertifiedKey::new(cert, signature_alg));

        self.cache.write().unwrap().insert(domain.clone(), Arc::clone(&ck));
        tracing::debug!("Stored certificate for {} in cache", domain);

        Some(ck)
    }
}
