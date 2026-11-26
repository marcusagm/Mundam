//! Authorizes asset data operations originating in the host-rendered Inspector.

use super::{inspector::verify_approved_package, PluginService};
use crate::core::{
    plugins::{
        asset_data::{
            PluginAssetDataLedger, PluginAssetFieldKey, PluginBooleanEdit, PluginBooleanWrite,
            PluginBooleanWriteOutcome, PluginInspectorEditResult, PluginInspectorFieldState,
            PluginInspectorValue, PluginSectionTarget,
        },
        presentation::LockedPluginPackage,
    },
    AppError, AppResult,
};
use mundam_plugin_sdk::{FieldType, Permission};

struct ResolvedField {
    key: PluginAssetFieldKey,
    schema_version: u32,
    default: bool,
    writable: bool,
}

impl PluginService {
    /// Loads values and editability from the active package's declared Inspector bindings.
    ///
    /// # Errors
    /// Rejects stale, unapproved or invalid requests and propagates storage failures.
    pub async fn read_inspector_values(
        &self,
        ledger: &dyn PluginAssetDataLedger,
        catalog: &str,
        target: PluginSectionTarget,
    ) -> AppResult<Vec<PluginInspectorFieldState>> {
        let (approval, fields) = self.resolve_inspector_fields(catalog, target).await?;
        let mut values = Vec::with_capacity(fields.len());
        for field in fields {
            let state = match ledger.read_plugin_boolean(&field.key).await? {
                Some(stored) if stored.schema_version != field.schema_version => {
                    PluginInspectorValue::SchemaMismatch
                }
                Some(stored) => PluginInspectorValue::Available {
                    value: stored.value,
                    revision: stored.revision.to_string(),
                    writable: field.writable,
                },
                None => PluginInspectorValue::Available {
                    value: field.default,
                    revision: "0".into(),
                    writable: field.writable,
                },
            };
            values.push(PluginInspectorFieldState {
                field_id: field.key.field_id,
                state,
            });
        }
        drop(approval);
        Ok(values)
    }

    /// Writes one declared boolean while holding the exact package approval through commit.
    ///
    /// # Errors
    /// Rejects missing, read-only, revoked or stale bindings, invalid revisions and storage failures.
    pub async fn save_inspector_value(
        &self,
        ledger: &dyn PluginAssetDataLedger,
        catalog: &str,
        edit: PluginBooleanEdit,
    ) -> AppResult<PluginInspectorEditResult> {
        let expected_revision = edit
            .expected_revision
            .parse::<i64>()
            .ok()
            .filter(|&revision| revision >= 0 && revision.to_string() == edit.expected_revision)
            .ok_or_else(|| AppError::ValidationFailed("Invalid plugin field revision".into()))?;
        let (approval, fields) = self.resolve_inspector_fields(catalog, edit.target).await?;
        let field = fields
            .into_iter()
            .find(|field| field.key.field_id == edit.field_id && field.writable)
            .ok_or_else(|| AppError::Forbidden("Plugin Inspector field is not editable".into()))?;
        let outcome = ledger
            .write_plugin_boolean(
                &field.key,
                PluginBooleanWrite {
                    schema_version: field.schema_version,
                    expected_revision,
                    value: edit.value,
                },
            )
            .await?;
        drop(approval);
        Ok(match outcome {
            PluginBooleanWriteOutcome::Stored(stored) => PluginInspectorEditResult::Saved {
                value: stored.value,
                revision: stored.revision.to_string(),
            },
            PluginBooleanWriteOutcome::Conflict { .. } => PluginInspectorEditResult::Conflict,
            PluginBooleanWriteOutcome::SchemaMismatch { .. } => {
                PluginInspectorEditResult::SchemaMismatch
            }
        })
    }

    async fn resolve_inspector_fields(
        &self,
        catalog: &str,
        target: PluginSectionTarget,
    ) -> AppResult<(Box<dyn LockedPluginPackage>, Vec<ResolvedField>)> {
        let executor = self
            .executor
            .filter(|_| !self.safe_mode)
            .ok_or_else(|| AppError::Forbidden("Plugin Inspector access is suspended".into()))?;
        let approval = self
            .registry
            .lock_approved_package(catalog, &target.plugin_id)
            .await?
            .ok_or_else(|| AppError::Forbidden("Plugin is not approved in this catalog".into()))?;
        let api_version = self.api_version.clone();
        tokio::task::spawn_blocking(move || {
            let package = approval.package();
            if package.revision() != target.package_revision
                || !package.permissions.contains(&Permission::SelectionRead)
                || !package.permissions.contains(&Permission::OwnDataRead)
            {
                return Err(AppError::Forbidden(
                    "Plugin Inspector approval changed".into(),
                ));
            }
            let verified = verify_approved_package(package, executor, &api_version)?;
            let contributions = &verified.manifest().manifest().contributes;
            let section = contributions
                .inspector_sections
                .iter()
                .find(|section| {
                    section.id == target.section_id && section.when.selection_count == 1
                })
                .ok_or_else(|| {
                    AppError::Forbidden("Plugin Inspector section is unavailable".into())
                })?;
            let can_write = package.permissions.contains(&Permission::OwnDataWrite);
            let fields = section
                .fields
                .iter()
                .filter_map(|binding| {
                    let field = contributions.asset_fields.iter().find(|field| {
                        field.id == binding.field && matches!(field.field_type, FieldType::Boolean)
                    })?;
                    Some(ResolvedField {
                        key: PluginAssetFieldKey {
                            asset_id: target.asset_id.clone(),
                            plugin_id: package.id.clone(),
                            publisher_fingerprint: package.publisher.fingerprint,
                            field_id: field.id.clone(),
                        },
                        schema_version: field.schema_version,
                        default: field.default,
                        writable: can_write && !binding.read_only,
                    })
                })
                .collect();
            Ok((approval, fields))
        })
        .await
        .map_err(|_| AppError::Internal("Plugin field verification task failed".into()))?
    }
}
