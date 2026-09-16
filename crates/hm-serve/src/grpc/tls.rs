use std::io::Cursor;
use std::sync::Arc;
use tokio_rustls::rustls;

#[derive(Clone)]
pub struct TlsIdentity {
    pub certificate_pem: Vec<u8>,
    pub private_key_pem: Vec<u8>,
    pub client_ca_pem: Vec<u8>,
}

impl TlsIdentity {
    pub(crate) fn tonic_config(
        &self,
    ) -> Result<tonic::transport::ServerTlsConfig, Box<dyn std::error::Error + Send + Sync>> {
        self.rustls_config()?;
        Ok(tonic::transport::ServerTlsConfig::new()
            .identity(tonic::transport::Identity::from_pem(
                &self.certificate_pem,
                &self.private_key_pem,
            ))
            .client_ca_root(tonic::transport::Certificate::from_pem(&self.client_ca_pem)))
    }

    pub(crate) fn rustls_config(
        &self,
    ) -> Result<Arc<rustls::ServerConfig>, Box<dyn std::error::Error + Send + Sync>> {
        let certificates = rustls_pemfile::certs(&mut Cursor::new(&self.certificate_pem))
            .collect::<Result<Vec<_>, _>>()?;
        let key = rustls_pemfile::private_key(&mut Cursor::new(&self.private_key_pem))?
            .ok_or("missing TLS private key")?;
        let mut roots = rustls::RootCertStore::empty();
        for cert in rustls_pemfile::certs(&mut Cursor::new(&self.client_ca_pem)) {
            roots.add(cert?)?;
        }
        if roots.is_empty() {
            return Err("client CA is mandatory".into());
        }
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(roots),
            provider.clone(),
        )
        .build()?;
        let mut config = rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()?
            .with_client_cert_verifier(verifier)
            .with_single_cert(certificates, key)?;
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        Ok(Arc::new(config))
    }
}
