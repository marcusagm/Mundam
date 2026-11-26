//! Projects trusted plugin declarations into the host presentation contract.

use super::{check_signed_package, PluginService};
use crate::core::{
    plugins::{
        inspector::{InspectorFieldKind, PluginInspectorField, PluginInspectorSection},
        presentation::{ApprovedPluginPackage, PluginCommand, PluginPresentation, PluginScreen},
    },
    AppError, AppResult,
};
use mundam_plugin_sdk::{Control, Executor, Manifest, Version};

impl PluginService {
    /// Returns approved presentation definitions for the active catalog.
    ///
    /// # Errors
    /// Propagates registry or verification task failures. A corrupt individual
    /// package is excluded so it cannot suppress other valid contributions.
    pub async fn presentation(&self, catalog_id: &str) -> AppResult<PluginPresentation> {
        let Some(executor) = self.executor.filter(|_| !self.safe_mode) else {
            return Ok(PluginPresentation::default());
        };
        let packages = self.registry.approved_packages(catalog_id).await?;
        if packages.is_empty() {
            return Ok(PluginPresentation::default());
        }
        let api_version = self.api_version.clone();
        tokio::task::spawn_blocking(move || {
            let mut presentation = PluginPresentation::default();
            for package in &packages {
                project_package(&mut presentation, package, executor, &api_version);
            }
            presentation
        })
        .await
        .map_err(|_| AppError::Internal("Plugin presentation task failed".into()))
    }
}

fn project_package(
    presentation: &mut PluginPresentation,
    package: &ApprovedPluginPackage,
    executor: Executor,
    api_version: &Version,
) {
    let Ok(verified) = check_signed_package(&package.bytes, &package.publisher) else {
        return;
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
        return;
    }
    presentation.sections.extend(sections(declaration));
    presentation
        .commands
        .extend(
            declaration
                .contributes
                .commands
                .iter()
                .map(|command| PluginCommand {
                    plugin_id: declaration.id.clone(),
                    plugin_name: declaration.name.clone(),
                    command_id: command.id.clone(),
                    title: command.title.clone(),
                    action: command.action.clone(),
                    suggested_shortcut: command.suggested_shortcut.clone(),
                }),
        );
    presentation
        .screens
        .extend(
            declaration
                .contributes
                .screens
                .iter()
                .map(|screen| PluginScreen {
                    plugin_id: declaration.id.clone(),
                    plugin_name: declaration.name.clone(),
                    screen_id: screen.id.clone(),
                    title: screen.title.clone(),
                    blocks: screen.blocks.clone(),
                }),
        );
}

fn sections(declaration: &Manifest) -> impl Iterator<Item = PluginInspectorSection> + '_ {
    declaration
        .contributes
        .inspector_sections
        .iter()
        .filter(|section| section.when.selection_count == 1)
        .map(move |section| PluginInspectorSection {
            plugin_id: declaration.id.clone(),
            plugin_name: declaration.name.clone(),
            section_id: section.id.clone(),
            title: section.title.clone(),
            viewport_action_label: section.viewport_action_label.clone(),
            sidebar_link_label: section.sidebar_link_label.clone(),
            title_bar_action_label: section.title_bar_action_label.clone(),
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
}
