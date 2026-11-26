//! Projects trusted inspector declarations into a limited host presentation contract.

use super::{check_signed_package, PluginService};
use crate::core::{
    plugins::inspector::{
        ActiveInspectorPackage, InspectorFieldKind, PluginInspectorField, PluginInspectorSection,
    },
    AppError, AppResult,
};
use mundam_plugin_sdk::{Control, Executor, Permission, Version};

impl PluginService {
    /// Returns approved sections and their navigation declarations for the active catalog.
    ///
    /// # Errors
    /// Propagates registry or verification task failures. A corrupt individual
    /// package is excluded so it cannot suppress other valid sections.
    pub async fn inspector_sections(
        &self,
        catalog_id: &str,
    ) -> AppResult<Vec<PluginInspectorSection>> {
        let Some(executor) = self.executor.filter(|_| !self.safe_mode) else {
            return Ok(Vec::new());
        };
        let packages = self.registry.active_inspector_packages(catalog_id).await?;
        if packages.is_empty() {
            return Ok(Vec::new());
        }
        let api_version = self.api_version.clone();
        tokio::task::spawn_blocking(move || {
            packages
                .iter()
                .flat_map(|package| sections(package, executor, &api_version))
                .collect()
        })
        .await
        .map_err(|_| AppError::Internal("Plugin inspector projection task failed".into()))
    }
}

fn sections(
    package: &ActiveInspectorPackage,
    executor: Executor,
    api_version: &Version,
) -> Vec<PluginInspectorSection> {
    if ![Permission::SelectionRead, Permission::OwnDataRead]
        .iter()
        .all(|permission| package.permissions.contains(permission))
    {
        return Vec::new();
    }
    let Ok(verified) = check_signed_package(&package.bytes, &package.publisher) else {
        return Vec::new();
    };
    let declaration = verified.manifest().manifest();
    if verified.package_digest() != package.digest
        || declaration.id != package.id
        || !verified.manifest().supports(executor, api_version)
        || !declaration
            .permissions
            .iter()
            .all(|permission| package.permissions.contains(permission))
    {
        return Vec::new();
    }

    declaration
        .contributes
        .inspector_sections
        .iter()
        .filter(|section| section.when.selection_count == 1)
        .map(|section| PluginInspectorSection {
            plugin_id: declaration.id.clone(),
            plugin_name: declaration.name.clone(),
            section_id: section.id.clone(),
            title: section.title.clone(),
            viewport_action_label: section.viewport_action_label.clone(),
            sidebar_link_label: section.sidebar_link_label.clone(),
            fields: section
                .fields
                .iter()
                .filter_map(|binding| {
                    let field = declaration
                        .contributes
                        .asset_fields
                        .iter()
                        .find(|field| field.id == binding.field)?;
                    let kind = match binding.control {
                        Control::Checkbox => InspectorFieldKind::Checkbox,
                    };
                    Some(PluginInspectorField {
                        field_id: field.id.clone(),
                        label: binding.label.clone(),
                        kind,
                        default_value: field.default,
                    })
                })
                .collect(),
        })
        .collect()
}
