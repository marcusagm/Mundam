//! SQLite consent transitions. Invitation codes, claim secrets and bearers are never persisted.
use crate::core::{
    imports::{
        destinations::DestinationGrant,
        pairing::{
            AuthenticatedImporter, ImportPairing, ImportScope, IssuedCredential, PairedImporter,
            PairingApproval, PairingClaim, PairingInvitation, PendingPairing,
        },
    },
    AppError, AppResult,
};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

const INVITATION_SECONDS: i64 = 300;
const CREDENTIAL_SECONDS: i64 = 30 * 24 * 3600;

/// Applies pairing against one catalog and the TLS identity provisioned by the host.
pub struct SqliteImportPairing<'pool> {
    pool: &'pool SqlitePool,
    server_key: String,
}

impl<'pool> SqliteImportPairing<'pool> {
    /// Creates the adapter using the digest of the active server's public key.
    pub fn new(pool: &'pool SqlitePool, server_key_digest: [u8; 32]) -> Self {
        Self {
            pool,
            server_key: blake3::Hash::from_bytes(server_key_digest)
                .to_hex()
                .to_string(),
        }
    }
}

#[async_trait]
impl ImportPairing for SqliteImportPairing<'_> {
    async fn invite(&self) -> AppResult<PairingInvitation> {
        let now = Utc::now().timestamp();
        let expires_at = now + INVITATION_SECONDS;
        let code = new_secret();
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("DELETE FROM import_pairings WHERE expires_at <= ?1 OR server_key != ?2")
            .bind(now)
            .bind(&self.server_key)
            .execute(&mut *transaction)
            .await?;
        let inserted = sqlx::query(
            "INSERT INTO import_pairings (id, server_key, expires_at, phase, verifier)
             SELECT ?1, ?2, ?3, 'invited', ?4 WHERE
                 (SELECT COUNT(*) FROM import_pairings WHERE phase != 'claimed') < 32",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&self.server_key)
        .bind(expires_at)
        .bind(verifier(&code)?)
        .execute(&mut *transaction)
        .await?;
        if inserted.rows_affected() == 0 {
            return Err(AppError::ValidationFailed(
                "Too many open pairing invitations".into(),
            ));
        }
        transaction.commit().await?;
        Ok(PairingInvitation { code, expires_at })
    }

    async fn request(
        &self,
        code: &str,
        name: &str,
        scopes: &[ImportScope],
    ) -> AppResult<PairingClaim> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err(AppError::ValidationFailed("Invalid importer name".into()));
        }
        let scopes = scope_mask(scopes)?;
        let secret = new_secret();
        let request_id = sqlx::query_scalar(
            "UPDATE import_pairings SET phase = 'pending', verifier = ?1, client_name = ?2, scopes = ?3
             WHERE phase = 'invited' AND verifier = ?4 AND server_key = ?5 AND expires_at > unixepoch()
             RETURNING id",
        )
        .bind(verifier(&secret)?).bind(name).bind(scopes).bind(verifier(code)?)
        .bind(&self.server_key)
        .fetch_optional(self.pool).await?.ok_or_else(pairing_unavailable)?;
        Ok(PairingClaim { request_id, secret })
    }

    async fn pending(&self) -> AppResult<Vec<PendingPairing>> {
        let rows: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT id, client_name, scopes FROM import_pairings
             WHERE phase = 'pending' AND server_key = ?1 AND expires_at > unixepoch()
             ORDER BY expires_at, id",
        )
        .bind(&self.server_key)
        .fetch_all(self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, client_name, scopes)| PendingPairing {
                id,
                client_name,
                requested_scopes: scopes_from_mask(scopes),
            })
            .collect())
    }

    async fn clients(&self) -> AppResult<Vec<PairedImporter>> {
        let rows: Vec<(String, String, i64, String)> = sqlx::query_as(
            "SELECT id, name, scopes, roots FROM import_clients
             WHERE server_key = ?1 AND verifier IS NOT NULL AND expires_at > unixepoch()
             ORDER BY name, id",
        )
        .bind(&self.server_key)
        .fetch_all(self.pool)
        .await?;
        rows.into_iter()
            .map(|(id, client_name, scopes, roots)| {
                Ok(PairedImporter {
                    id,
                    client_name,
                    scopes: scopes_from_mask(scopes),
                    root_ids: serde_json::from_str(&roots).map_err(json_error)?,
                })
            })
            .collect()
    }

    async fn approve(&self, id: &str, approval: PairingApproval<'_>) -> AppResult<()> {
        let scopes = scope_mask(approval.scopes)?;
        let needs_roots =
            scopes & (ImportScope::ReadFolders.mask() | ImportScope::ImportAssets.mask()) != 0;
        if approval.root_ids.len() > 100 || needs_roots == approval.root_ids.is_empty() {
            return Err(AppError::ValidationFailed(
                "Invalid root selection for the approved scopes".into(),
            ));
        }
        let mut roots = approval.root_ids.to_vec();
        roots.sort_unstable();
        roots.dedup();
        let roots = serde_json::to_string(&roots).map_err(json_error)?;
        let updated = sqlx::query(
            "UPDATE import_pairings SET phase = 'approved', scopes = ?2, roots = ?3
             WHERE id = ?1 AND phase = 'pending' AND expires_at > unixepoch() AND server_key = ?4
               AND (scopes & ?2) = ?2
               AND EXISTS (SELECT 1 FROM import_preferences WHERE catalog_id = ?5)
               AND NOT EXISTS (
                   SELECT 1 FROM json_each(?3) AS selected
                   WHERE NOT EXISTS (
                       SELECT 1 FROM folders WHERE id = selected.value AND parent_id IS NULL
                   )
               )",
        )
        .bind(id)
        .bind(scopes)
        .bind(roots)
        .bind(&self.server_key)
        .bind(approval.catalog_id)
        .execute(self.pool)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(pairing_unavailable());
        }
        Ok(())
    }

    async fn reject(&self, id: &str) -> AppResult<()> {
        sqlx::query(
            "DELETE FROM import_pairings WHERE id = ?1 AND server_key = ?2 AND phase IN ('pending','approved')",
        ).bind(id).bind(&self.server_key).execute(self.pool).await?;
        Ok(())
    }

    async fn redeem(&self, id: &str, secret: &str) -> AppResult<IssuedCredential> {
        let now = Utc::now().timestamp();
        let bearer = new_secret();
        let expires_at = now + CREDENTIAL_SECONDS;
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let claimed = sqlx::query(
            "UPDATE import_pairings SET phase = 'claimed', verifier = NULL
             WHERE id = ?1 AND verifier = ?2 AND phase = 'approved' AND expires_at > unixepoch() AND server_key = ?3",
        ).bind(id).bind(verifier(secret)?).bind(&self.server_key)
        .execute(&mut *transaction).await?;
        if claimed.rows_affected() == 0 {
            return Err(pairing_unavailable());
        }
        sqlx::query(
            "INSERT INTO import_clients (id, name, server_key, verifier, expires_at, scopes, roots)
             SELECT id, client_name, server_key, ?2, ?3, scopes, roots FROM import_pairings WHERE id = ?1",
        ).bind(id).bind(verifier(&bearer)?).bind(expires_at).execute(&mut *transaction).await?;
        sqlx::query("DELETE FROM import_pairings WHERE id = ?1")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(IssuedCredential {
            client_id: id.to_owned(),
            bearer,
            expires_at,
        })
    }

    async fn authenticate(&self, bearer: &str) -> AppResult<AuthenticatedImporter> {
        let (client_id, version, catalog_id, scopes, roots): (String, u32, String, i64, String) =
            sqlx::query_as(
                "SELECT id, grant_version, catalog_id, scopes, roots
                 FROM import_clients, import_preferences
                 WHERE verifier = ?1 AND server_key = ?2 AND expires_at > unixepoch()",
            )
            .bind(verifier(bearer)?)
            .bind(&self.server_key)
            .fetch_optional(self.pool)
            .await?
            .ok_or_else(pairing_unavailable)?;
        let root_ids = serde_json::from_str(&roots).map_err(json_error)?;
        Ok(AuthenticatedImporter::new(
            client_id,
            u64::from(version),
            DestinationGrant {
                catalog_id,
                root_ids,
            },
            scopes_from_mask(scopes),
            self.server_key.clone(),
        ))
    }

    async fn rotate(&self, previous: &str) -> AppResult<IssuedCredential> {
        let now = Utc::now().timestamp();
        let expires_at = now + CREDENTIAL_SECONDS;
        let bearer = new_secret();
        let client_id = sqlx::query_scalar(
            "UPDATE import_clients SET verifier = ?1, expires_at = ?2, grant_version = grant_version + 1
             WHERE verifier = ?3 AND expires_at > unixepoch() AND server_key = ?4 RETURNING id",
        ).bind(verifier(&bearer)?).bind(expires_at).bind(verifier(previous)?).bind(&self.server_key)
        .fetch_optional(self.pool).await?.ok_or_else(pairing_unavailable)?;
        Ok(IssuedCredential {
            client_id,
            bearer,
            expires_at,
        })
    }

    async fn revoke(&self, catalog_id: &str, client_id: &str) -> AppResult<()> {
        sqlx::query(
            "UPDATE import_clients SET verifier = NULL, grant_version = grant_version + 1
             WHERE id = ?1 AND verifier IS NOT NULL
               AND EXISTS (SELECT 1 FROM import_preferences WHERE catalog_id = ?2)",
        )
        .bind(client_id)
        .bind(catalog_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}

fn new_secret() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn verifier(secret: &str) -> AppResult<String> {
    if secret.len() != 64 || !secret.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(pairing_unavailable());
    }
    Ok(blake3::Hash::from_bytes(blake3::derive_key(
        "mundam.imports.credentials.v1",
        secret.as_bytes(),
    ))
    .to_hex()
    .to_string())
}

fn scope_mask(scopes: &[ImportScope]) -> AppResult<i64> {
    let mask = scopes.iter().fold(0, |mask, scope| mask | scope.mask());
    if mask == 0
        || (scopes.contains(&ImportScope::AssignTagsOnImport)
            && !scopes.contains(&ImportScope::ImportAssets))
    {
        return Err(AppError::ValidationFailed(
            "Invalid importer permissions".into(),
        ));
    }
    Ok(mask)
}

fn scopes_from_mask(mask: i64) -> Vec<ImportScope> {
    ImportScope::ALL
        .into_iter()
        .filter(|scope| mask & scope.mask() != 0)
        .collect()
}

fn pairing_unavailable() -> AppError {
    AppError::Forbidden("Pairing or credential unavailable".into())
}

fn json_error(error: serde_json::Error) -> AppError {
    AppError::Internal(error.to_string())
}

#[cfg(test)]
mod tests;
