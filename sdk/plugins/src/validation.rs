//! Semantic checks that require cross-references, version ordering or normalization.

use crate::{
    Manifest, Permission, ValidatedManifest, ValidationError, Version, MAX_MANIFEST_BYTES,
};
use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug)]
pub(crate) struct ApiInterval {
    minimum: Version,
    exclusive_maximum: Version,
}

impl ApiInterval {
    fn parse(value: &str) -> Result<Self, ValidationError> {
        fn bound(value: &str, prefix: &str) -> Option<Version> {
            let version = Version::parse(value.strip_prefix(prefix)?).ok()?;
            (version.pre.is_empty() && version.build.is_empty()).then_some(version)
        }

        let interval = value.split_once(' ').and_then(|(minimum, maximum)| {
            Some(Self {
                minimum: bound(minimum, ">=")?,
                exclusive_maximum: bound(maximum, "<")?,
            })
        });
        interval
            .filter(|interval| interval.minimum < interval.exclusive_maximum)
            .ok_or_else(|| {
                ValidationError::new(
                    "engines.mundamApi",
                    "expected an increasing stable version interval such as >=1.0.0 <2.0.0",
                )
            })
    }

    pub(crate) fn contains(&self, version: &Version) -> bool {
        use std::cmp::Ordering;
        version.pre.is_empty()
            && version.cmp_precedence(&self.minimum) != Ordering::Less
            && version.cmp_precedence(&self.exclusive_maximum) == Ordering::Less
    }
}

/// Parses an untrusted manifest and returns the same validated declaration a host can consume.
///
/// # Errors
/// Rejects oversized JSON, unknown fields or values, invalid metadata, unsupported
/// contributions, unresolved references and insufficient required declarations.
pub fn parse_manifest(bytes: &[u8]) -> Result<ValidatedManifest, ValidationError> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return invalid("manifest", "exceeds 64 KiB");
    }
    let mut manifest: Manifest = serde_json::from_slice(bytes)
        .map_err(|error| ValidationError::new("manifest", error.to_string()))?;
    if manifest.manifest_version != 1 {
        return invalid("manifestVersion", "only manifest version 1 is supported");
    }
    let api = ApiInterval::parse(&manifest.engines.mundam_api)?;
    validate_metadata(&mut manifest)?;
    validate_contributions(&manifest)?;
    Ok(ValidatedManifest { manifest, api })
}

fn invalid<T>(field: &'static str, message: &str) -> Result<T, ValidationError> {
    Err(ValidationError::new(field, message))
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value == value.trim()
        && value.chars().count() <= maximum
        && !value.chars().any(|character| {
            character.is_control() || matches!(character, '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{feff}')
        })
}

fn valid_slug(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && value.len() <= 32
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// Checks the namespace grammar shared by manifest authors and publisher registries.
pub fn valid_namespace(value: &str) -> bool {
    value.len() <= 128 && value.contains('.') && value.split('.').all(valid_slug)
}

fn valid_icon_path(path: &str) -> bool {
    path.strip_prefix("assets/")
        .and_then(|file| file.strip_suffix(".png"))
        .is_some_and(|stem| {
            let reserved = matches!(stem, "con" | "prn" | "aux" | "nul")
                || (stem.len() == 4
                    && (stem.starts_with("com") || stem.starts_with("lpt"))
                    && stem.as_bytes()[3].is_ascii_digit());
            valid_slug(stem) && !reserved
        })
}

fn validate_metadata(manifest: &mut Manifest) -> Result<(), ValidationError> {
    if !valid_namespace(&manifest.publisher) {
        return invalid("publisher", "expected a reverse-domain namespace");
    }
    if !valid_namespace(&manifest.id)
        || !manifest
            .id
            .strip_prefix(&manifest.publisher)
            .is_some_and(|suffix| suffix.starts_with('.'))
    {
        return invalid("id", "expected an ID under the publisher's namespace");
    }
    for (field, value, maximum) in [
        ("name", manifest.name.as_str(), 80),
        ("description", manifest.description.as_str(), 240),
        ("license", manifest.license.as_str(), 128),
    ] {
        if !valid_text(value, maximum) {
            return invalid(field, "expected nonempty plain text within the field limit");
        }
    }
    if manifest.version.len() > 64 || Version::parse(&manifest.version).is_err() {
        return invalid(
            "version",
            "expected a SemVer release of at most 64 characters",
        );
    }
    if !valid_icon_path(&manifest.icon) {
        return invalid("icon", "expected a portable path such as assets/review.png");
    }
    let targets: HashSet<_> = manifest.targets.iter().collect();
    if targets.is_empty() || targets.len() != manifest.targets.len() {
        return invalid(
            "targets",
            "expected one or more unique platform/architecture pairs",
        );
    }
    if !(1..=12).contains(&manifest.keywords.len())
        || !manifest
            .keywords
            .iter()
            .all(|keyword| valid_text(keyword, 32))
    {
        return invalid(
            "keywords",
            "expected 1 to 12 nonempty terms of at most 32 characters",
        );
    }
    let mut seen = HashSet::new();
    manifest
        .keywords
        .retain(|keyword| seen.insert(keyword.to_lowercase().nfc().collect::<String>()));

    let mut permissions = HashSet::new();
    if !manifest
        .permissions
        .iter()
        .chain(&manifest.optional_permissions)
        .all(|permission| permissions.insert(permission))
    {
        return invalid(
            "permissions",
            "permissions must be unique across both lists",
        );
    }
    Ok(())
}

fn validate_contributions(manifest: &Manifest) -> Result<(), ValidationError> {
    let contributions = &manifest.contributes;
    let mut fields = HashSet::new();
    if !(1..=32).contains(&contributions.asset_fields.len())
        || !contributions.asset_fields.iter().all(|field| {
            valid_slug(&field.id) && field.schema_version > 0 && fields.insert(field.id.as_str())
        })
    {
        return invalid(
            "contributes.assetFields",
            "expected 1 to 32 distinct field IDs with a positive schema version",
        );
    }

    if !(1..=8).contains(&contributions.inspector_sections.len()) {
        return invalid(
            "contributes.inspectorSections",
            "expected 1 to 8 inspector sections",
        );
    }
    for permission in [Permission::SelectionRead, Permission::OwnDataRead] {
        if !manifest.permissions.contains(&permission) {
            return invalid(
                "permissions",
                "inspector sections require selection.read and assetData.own.read",
            );
        }
    }

    let mut section_ids = HashSet::new();
    for section in &contributions.inspector_sections {
        let mut bindings = HashSet::new();
        for (field, label) in [
            (
                "contributes.inspectorSections.viewportActionLabel",
                &section.viewport_action_label,
            ),
            (
                "contributes.inspectorSections.sidebarLinkLabel",
                &section.sidebar_link_label,
            ),
            (
                "contributes.inspectorSections.titleBarActionLabel",
                &section.title_bar_action_label,
            ),
        ] {
            if label.as_ref().is_some_and(|label| !valid_text(label, 80)) {
                return invalid(
                    field,
                    "expected a plain-text navigation label of up to 80 characters",
                );
            }
        }
        if !valid_slug(&section.id)
            || !section_ids.insert(&section.id)
            || !valid_text(&section.title, 80)
            || section.when.selection_count != 1
            || !(1..=16).contains(&section.fields.len())
            || !section.fields.iter().all(|binding| {
                fields.contains(binding.field.as_str())
                    && bindings.insert(&binding.field)
                    && valid_text(&binding.label, 80)
            })
        {
            return invalid(
                "contributes.inspectorSections",
                "expected unique section IDs, single selection and unique local field bindings with labels",
            );
        }
        if section.fields.iter().any(|binding| !binding.read_only)
            && !manifest.permissions.contains(&Permission::OwnDataWrite)
        {
            return invalid(
                "permissions",
                "editable controls require assetData.own.write",
            );
        }
    }
    Ok(())
}
