//! Phase-zero HTTPS probe. A test controller supplies a disposable identity and two clients
//! over stdin. Only the controller's staging database is opened; no desktop services are used.
mod routes;
mod staging;

use axum::Router;
use base64::{engine::general_purpose::STANDARD, Engine};
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{error::Error, io::Read, net::Ipv4Addr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::Semaphore, time::timeout};
use tokio_rustls::{
    rustls::{
        self,
        pki_types::{CertificateDer, PrivatePkcs8KeyDer},
    },
    TlsAcceptor,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProbeIdentity {
    certificate_der: String,
    private_key_der: String,
    owner_token: String,
    other_token: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let database = std::env::args_os()
        .nth(1)
        .ok_or("Missing test database path")?;
    let mut input = Vec::new();
    std::io::stdin().take(32_769).read_to_end(&mut input)?;
    if input.len() > 32_768 {
        return Err("Oversized configuration".into());
    }
    let identity: ProbeIdentity = serde_json::from_slice(&input)?;
    if identity.owner_token.len() < 32
        || identity.other_token.len() < 32
        || identity.owner_token == identity.other_token
    {
        return Err("Two distinct test credentials are required".into());
    }
    let tls = rustls::ServerConfig::builder_with_provider(
        rustls::crypto::ring::default_provider().into(),
    )
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(
        vec![CertificateDer::from(
            STANDARD.decode(identity.certificate_der)?,
        )],
        PrivatePkcs8KeyDer::from(STANDARD.decode(identity.private_key_der)?).into(),
    )?;
    let acceptor = TlsAcceptor::from(Arc::new(tls));
    let pool = staging::open(database.as_ref()).await?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    let address = listener.local_addr()?;
    let app = routes::router(routes::State {
        pool,
        authority: address.to_string(),
        owner_hash: Sha256::digest(identity.owner_token.as_bytes()).into(),
        other_hash: Sha256::digest(identity.other_token.as_bytes()).into(),
    });
    println!("{}", serde_json::json!({ "port": address.port() }));
    // Bound the lifetime of the test listener even if its controller disappears.
    timeout(Duration::from_secs(120), serve(listener, acceptor, app)).await??;
    Ok(())
}

async fn serve(listener: TcpListener, acceptor: TlsAcceptor, app: Router) -> std::io::Result<()> {
    let capacity = Arc::new(Semaphore::new(8));
    loop {
        let (stream, _) = listener.accept().await?;
        let Ok(permit) = capacity.clone().try_acquire_owned() else {
            continue;
        };
        let acceptor = acceptor.clone();
        let service = TowerToHyperService::new(app.clone());
        tokio::spawn(async move {
            let _permit = permit;
            let _ = timeout(Duration::from_secs(15), async {
                if let Ok(channel) = acceptor.accept(stream).await {
                    let _ = hyper::server::conn::http1::Builder::new()
                        .keep_alive(false)
                        .serve_connection(TokioIo::new(channel), service)
                        .await;
                }
            })
            .await;
        });
    }
}
