//! Exercises the real import router and TLS listener against a disposable catalog.
//! Stdin simulates trusted desktop consent. Network clients only receive HTTP routes.
use base64::{engine::general_purpose::STANDARD, Engine};
use mundam_lib::{
    core::imports::{
        destinations::ImportDestinations,
        pairing::{ImportPairing, ImportScope, PairingApproval},
    },
    delivery::external_api::listener::ImportHttpsListener,
    infra::database::{
        import_destinations::SqliteImportDestinations, import_pairing::SqliteImportPairing,
        manager::DbManager,
    },
};
use serde::Deserialize;
use serde_json::json;
use std::{error::Error, path::Path, time::Duration};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use tokio_util::sync::CancellationToken;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Identity {
    certificate_der: String,
    private_key_der: String,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum DesktopAction {
    Approve {
        id: String,
        scopes: Vec<ImportScope>,
    },
    Revoke {
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tokio::time::timeout(Duration::from_secs(120), run()).await?
}

async fn run() -> Result<(), Box<dyn Error>> {
    let database_path = std::env::args_os()
        .nth(1)
        .ok_or("Missing temporary catalog path")?;
    let database = DbManager::new(Path::new(&database_path)).await?;
    let pool = database.pool();
    sqlx::raw_sql(
        "INSERT OR IGNORE INTO folders (id, name, path) VALUES
            ('references','References','/probe/references'), ('private','Private','/probe/private');
         INSERT OR IGNORE INTO tags (id, name) VALUES ('portrait','Portrait');",
    )
    .execute(pool)
    .await?;
    let catalog = SqliteImportDestinations(pool).preference().await?;
    let mut commands = BufReader::new(tokio::io::stdin()).lines();
    let input = commands
        .next_line()
        .await?
        .ok_or("Missing temporary TLS identity")?;
    let identity: Identity = serde_json::from_str(&input)?;
    let server = ImportHttpsListener::bind(
        pool.clone(),
        CertificateDer::from(STANDARD.decode(identity.certificate_der)?),
        PrivatePkcs8KeyDer::from(STANDARD.decode(identity.private_key_der)?).into(),
    )
    .await?;
    let server_key = server.server_key_digest();
    let pairing = SqliteImportPairing::new(pool, server_key);
    let invitation = pairing.invite().await?;
    let cancellation = CancellationToken::new();
    let port = server.address().port();
    let service = tokio::spawn(server.serve(cancellation.clone()));
    println!(
        "{}",
        json!({
            "port": port,
            "serverKey": blake3::Hash::from_bytes(server_key).to_hex().to_string(),
            "code": invitation.code,
        })
    );
    let roots = ["references".into()];
    let result = async {
        while let Some(line) = commands.next_line().await? {
            let action: DesktopAction = serde_json::from_str(&line)?;
            let result = match action {
                DesktopAction::Approve { id, scopes } => {
                    pairing
                        .approve(
                            &id,
                            PairingApproval {
                                catalog_id: &catalog.catalog_id,
                                scopes: &scopes,
                                root_ids: &roots,
                            },
                        )
                        .await
                }
                DesktopAction::Revoke { id } => pairing.revoke(&catalog.catalog_id, &id).await,
            };
            println!("{}", json!({"ok": result.is_ok()}));
        }
        Ok(())
    }
    .await;
    cancellation.cancel();
    service.await??;
    database.close().await;
    result
}
