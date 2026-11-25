//! HTTPS transport for the import API, independent of desktop activation and discovery.
use crate::core::{AppError, AppResult};
use crate::infra::database::import_staging::SqliteImportStaging;
use axum::Router;
use hyper::server::conn::http1;
use hyper_util::{
    rt::{TokioIo, TokioTimer},
    service::TowerToHyperService,
};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinSet,
    time::timeout,
};
use tokio_rustls::{
    rustls::{
        self,
        pki_types::{CertificateDer, PrivateKeyDer},
        server::ParsedCertificate,
    },
    TlsAcceptor,
};
use tokio_util::sync::CancellationToken;

/// A bound import service. The owner publishes its endpoint through a trusted channel.
pub struct ImportHttpsListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    router: Router,
    address: SocketAddrV4,
    server_key: [u8; 32],
}

impl ImportHttpsListener {
    /// Binds an ephemeral port on IPv4 loopback and derives the pairing identity from the
    /// certificate actually served. Generates a fresh signing key for pagination cursors.
    ///
    /// # Errors
    /// Returns failure for invalid key material, unavailable randomness or a bind error.
    pub async fn bind(
        pool: SqlitePool,
        certificate: CertificateDer<'static>,
        private_key: PrivateKeyDer<'static>,
        staging: Option<SqliteImportStaging>,
    ) -> AppResult<Self> {
        let certificate_key = ParsedCertificate::try_from(&certificate)
            .map_err(tls_configuration_error)?
            .subject_public_key_info();
        let server_key = Sha256::digest(certificate_key.as_ref()).into();
        let provider = rustls::crypto::ring::default_provider();
        let mut cursor_key = [0_u8; 32];
        provider
            .secure_random
            .fill(&mut cursor_key)
            .map_err(|_| AppError::Internal("Could not generate the import cursor key".into()))?;
        let tls = rustls::ServerConfig::builder_with_provider(provider.into())
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(tls_configuration_error)?
            .with_no_client_auth()
            .with_single_cert(vec![certificate], private_key)
            .map_err(tls_configuration_error)?;
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let SocketAddr::V4(address) = listener.local_addr()? else {
            return Err(AppError::Internal(
                "Unexpected import listener address".into(),
            ));
        };
        let router = super::router(pool, address, server_key, cursor_key, staging)?;
        Ok(Self {
            listener,
            acceptor: TlsAcceptor::from(Arc::new(tls)),
            router,
            address,
            server_key,
        })
    }

    /// Returns the address clients must use in HTTPS requests.
    pub fn address(&self) -> SocketAddrV4 {
        self.address
    }

    /// Returns the SHA-256 digest of the certificate's SubjectPublicKeyInfo.
    pub fn server_key_digest(&self) -> [u8; 32] {
        self.server_key
    }

    /// Accepts and supervises connections until cancellation or a listener failure.
    ///
    /// # Errors
    /// Returns a listener or connection-task failure after closing remaining connections.
    pub async fn serve(self, cancellation: CancellationToken) -> AppResult<()> {
        let connection_cancel = cancellation.child_token();
        let mut connections = JoinSet::new();
        let outcome = loop {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => break Ok(()),
                joined = connections.join_next(), if !connections.is_empty() => {
                    if let Some(Err(error)) = joined {
                        break Err(AppError::Internal(format!("Import connection task failed: {error}")));
                    }
                }
                accepted = self.listener.accept() => {
                    let stream = match accepted {
                        Ok((stream, _)) => stream,
                        Err(error) => break Err(error.into()),
                    };
                    if connections.len() >= 16 {
                        continue;
                    }
                    connections.spawn(serve_connection(stream, self.acceptor.clone(), self.router.clone(), connection_cancel.clone()));
                }
            }
        };
        drop(self.listener);
        connection_cancel.cancel();
        if timeout(Duration::from_secs(3), async {
            while connections.join_next().await.is_some() {}
        })
        .await
        .is_err()
        {
            connections.shutdown().await;
        }
        outcome
    }
}

async fn serve_connection(
    stream: TcpStream,
    acceptor: TlsAcceptor,
    router: Router,
    cancellation: CancellationToken,
) {
    let channel = tokio::select! {
        biased;
        () = cancellation.cancelled() => return,
        result = timeout(Duration::from_secs(5), acceptor.accept(stream)) => match result {
            Ok(Ok(channel)) => channel,
            _ => return,
        },
    };
    let mut http = http1::Builder::new();
    http.keep_alive(false)
        .timer(TokioTimer::new())
        .header_read_timeout(Duration::from_secs(5));
    let connection = http.serve_connection(TokioIo::new(channel), TowerToHyperService::new(router));
    tokio::pin!(connection);
    let _ = timeout(Duration::from_secs(15), async {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => {
                connection.as_mut().graceful_shutdown();
                connection.await
            }
            result = &mut connection => result,
        }
    })
    .await;
}

fn tls_configuration_error(_: rustls::Error) -> AppError {
    AppError::ValidationFailed("Invalid import TLS certificate or private key".into())
}
