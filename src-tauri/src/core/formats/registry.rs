use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

/// The central registry for all supported file formats.
///
/// Routes file identification requests through a two-tier resolution strategy:
/// O(1) extension-based lookup for the common case, with an O(N) magic-byte
/// fallback for extensionless or misidentified files.
#[derive(Clone)]
pub struct FormatRegistry {
    /// Instant routing (O(1)) for 99% of cases using normalized extensions.
    by_extension: HashMap<String, Arc<dyn FormatProvider>>,
    /// Fast set of all supported extensions for filtering.
    supported_extensions: HashSet<String>,
    /// Passive fallback for binary formats without extensions or deep validation.
    deep_checkers: Vec<Arc<dyn FormatProvider>>,
}

/// Format Registry implementation.
impl FormatRegistry {
    /// Creates a new, empty FormatRegistry.
    pub fn new() -> Self {
        Self {
            by_extension: HashMap::new(),
            supported_extensions: HashSet::new(),
            deep_checkers: Vec::new(),
        }
    }

    /// Registers a provider in the registry.
    ///
    /// This will automatically index the provider by its supported extensions
    /// and add it to the deep checker fallback list.
    ///
    /// # Arguments
    ///
    /// * `provider` - The format provider to register.
    ///
    /// # Duplicate Detection
    ///
    /// If a provider is registered for an extension that is already claimed by
    /// another provider, a warning is emitted via `tracing::warn!`. The new
    /// provider overwrites the previous one for that extension.
    pub fn register(&mut self, provider: Arc<dyn FormatProvider>) {
        for extension in provider.supported_extensions() {
            let extension_lower = extension.to_lowercase();
            if let Some(existing_provider) = self.by_extension.get(&extension_lower) {
                tracing::warn!(
                    extension = %extension_lower,
                    existing_provider = %existing_provider.name(),
                    new_provider = %provider.name(),
                    "Extension conflict: overwriting existing provider registration"
                );
            }
            self.by_extension
                .insert(extension_lower.clone(), provider.clone());
            self.supported_extensions.insert(extension_lower);
        }
        self.deep_checkers.push(provider);
    }

    /// Registers a batch of providers in the registry.
    ///
    /// This is a convenience method for registering all providers collected
    /// from a category module's `collect_providers()` function. Providers are
    /// registered in the order they appear in the iterator.
    ///
    /// # Arguments
    ///
    /// * `providers` - An iterable of providers to register.
    pub fn register_batch(&mut self, providers: impl IntoIterator<Item = Arc<dyn FormatProvider>>) {
        for provider in providers {
            self.register(provider);
        }
    }

    /// Registers a fallback provider that will be checked last during
    /// magic-byte resolution.
    ///
    /// This method exists for semantic clarity at the call site, ensuring
    /// that the fallback provider is always the last resort in the deep
    /// checker chain.
    ///
    /// # Arguments
    ///
    /// * `fallback_provider` - The fallback provider to register as the
    ///   final deep checker.
    pub fn register_fallback(&mut self, fallback_provider: Arc<dyn FormatProvider>) {
        self.register(fallback_provider);
    }

    /// Resolves the most appropriate format provider for a given file.
    ///
    /// # Performance
    /// 1. Tries O(1) lookup via file extension.
    /// 2. If no extension matches or if the provider requires magic byte validation,
    ///    it falls back to O(N) magic byte inspection across all registered providers.
    ///
    /// # Arguments
    /// * `path` - The canonical file path.
    /// * `header` - A buffer containing the initial bytes of the file for magic byte validation.
    pub fn resolve(&self, path: &Path, header: &[u8]) -> Option<Arc<dyn FormatProvider>> {
        // 1. FAST ATTEMPT (O(1)): Cache by Extension
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        if let Some(ref ext) = extension {
            if let Some(provider) = self.by_extension.get(ext) {
                // Double check magic bytes if the provider supports it and we have header content
                if header.is_empty() || provider.supports_magic_bytes(header) {
                    return Some(provider.clone());
                }
            }
        }

        // 2. SLOW FALLBACK (O(N)): Magic Bytes inferred (For files without extension or ghost extensions)
        if !header.is_empty() {
            // Use infer to get MIME and try to match it
            if let Some(kind) = infer::get(header) {
                let mime = kind.mime_type();

                // CRITICAL FIX: If infer says it's a generic format like TIFF or ZIP,
                // we prefer the extension-based provider because many pro formats use these containers.
                if (mime == "image/tiff"
                    || mime == "application/zip"
                    || mime == "application/octet-stream")
                    && extension.is_some()
                {
                    if let Some(provider) = extension
                        .as_ref()
                        .and_then(|ext| self.by_extension.get(ext))
                    {
                        return Some(provider.clone());
                    }
                }

                // Match by MIME across all providers
                if let Some(provider) = self.deep_checkers.iter().find(|p| {
                    p.supported_formats()
                        .iter()
                        .any(|f| f.mime_types.contains(&mime.to_string()))
                }) {
                    return Some(provider.clone());
                }
            }

            // Absolute fallback: direct magic byte check
            return self
                .deep_checkers
                .iter()
                .find(|provider| provider.supports_magic_bytes(header))
                .cloned();
        }

        None
    }

    /// Detects the granular format for a given path using extension-based routing.
    pub fn detect(&self, path: &Path) -> Option<SupportedFormat> {
        let extension = path.extension()?.to_str()?.to_lowercase();
        self.detect_by_extension(&extension)
    }

    /// Detects the granular format for a given extension.
    pub fn detect_by_extension(&self, extension: &str) -> Option<SupportedFormat> {
        let ext_lower = extension.to_lowercase();
        let provider = self.by_extension.get(&ext_lower)?;
        provider.supported_formats().into_iter().find(|sf| {
            sf.extensions
                .iter()
                .any(|ext| ext.to_lowercase() == ext_lower)
        })
    }

    /// Checks if the file extension is supported by the library.
    pub fn is_supported_extension(&self, extension: &str) -> bool {
        self.supported_extensions
            .contains(&extension.to_lowercase())
    }

    /// Returns a provider by its unique name.
    pub fn get_provider(&self, name: &str) -> Option<Arc<dyn FormatProvider>> {
        self.deep_checkers
            .iter()
            .find(|p| p.name() == name)
            .cloned()
    }

    /// Returns a list of all supported formats and their extensions.
    pub fn get_supported_formats(&self) -> Vec<SupportedFormat> {
        self.deep_checkers
            .iter()
            .flat_map(|provider| provider.supported_formats())
            .collect()
    }

    /// Resolves a provider primarily by its MIME type.
    pub fn resolve_by_mime(&self, mime: &str) -> Option<Arc<dyn FormatProvider>> {
        self.deep_checkers
            .iter()
            .find(|p| {
                p.supported_formats()
                    .iter()
                    .any(|f| f.mime_types.contains(&mime.to_string()))
            })
            .cloned()
    }

    /// Resolves a provider by its human-readable format name (e.g., "JPEG Image").
    pub fn resolve_by_format_name(&self, name: &str) -> Option<Arc<dyn FormatProvider>> {
        self.deep_checkers
            .iter()
            .find(|p| p.supported_formats().iter().any(|f| f.name == name))
            .cloned()
    }
}

impl Default for FormatRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Tests for the format registry.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::AppResult;
    use crate::core::formats::capabilities::MetadataCapability;
    use async_trait::async_trait;

    struct MockProvider;

    /// Tests the metadata capability.
    #[async_trait]
    impl MetadataCapability for MockProvider {
        async fn extract_technical(&self, _path: &Path) -> AppResult<serde_json::Value> {
            Ok(serde_json::json!({ "type": "mock" }))
        }
        async fn extract_semantic(&self, _path: &Path) -> AppResult<serde_json::Value> {
            Ok(serde_json::json!({ "tags": ["mock"] }))
        }
    }

    /// Tests the format provider.
    impl FormatProvider for MockProvider {
        fn name(&self) -> &'static str {
            "MOCK"
        }
        fn supported_extensions(&self) -> Vec<&'static str> {
            vec!["mock", "MOK"]
        }
        fn supports_magic_bytes(&self, header_bytes: &[u8]) -> bool {
            header_bytes.starts_with(b"MOCK")
        }
        fn metadata(&self) -> Option<&dyn MetadataCapability> {
            Some(self)
        }
    }

    /// Tests the format registry resolution.
    #[test]
    fn test_registry_resolution() -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = FormatRegistry::new();
        let provider = Arc::new(MockProvider);
        registry.register(provider.clone());

        // Test O(1) extension resolution
        let path = Path::new("test.mock");
        let resolved = registry.resolve(path, &[]);
        assert!(resolved.is_some());
        assert_eq!(
            resolved
                .ok_or("Expected a value in test_registry_resolution")?
                .name(),
            "MOCK"
        );

        // Test case-insensitivity
        let path = Path::new("test.MOK");
        let resolved = registry.resolve(path, &[]);
        assert!(resolved.is_some());

        // Test magic bytes fallback
        let path = Path::new("no_extension");
        let resolved = registry.resolve(path, b"MOCK_HEADER");
        assert!(resolved.is_some());
        assert_eq!(
            resolved
                .ok_or("Expected a value in test_registry_resolution")?
                .name(),
            "MOCK"
        );

        // Test failure
        let path = Path::new("test.unknown");
        let resolved = registry.resolve(path, b"RANDOM");
        assert!(resolved.is_none());

        Ok(())
    }

    /// Tests batch registration of multiple providers.
    #[test]
    fn test_register_batch() -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = FormatRegistry::new();
        let providers: Vec<Arc<dyn FormatProvider>> = vec![Arc::new(MockProvider)];
        registry.register_batch(providers);

        let path = Path::new("test.mock");
        let resolved = registry.resolve(path, &[]);
        assert!(resolved.is_some());
        assert_eq!(
            resolved
                .ok_or("Expected a value in test_register_batch")?
                .name(),
            "MOCK"
        );

        Ok(())
    }

    /// Tests that register_fallback works identically to register.
    #[test]
    fn test_register_fallback() -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = FormatRegistry::new();
        let provider = Arc::new(MockProvider);
        registry.register_fallback(provider);

        let path = Path::new("test.mock");
        let resolved = registry.resolve(path, &[]);
        assert!(resolved.is_some());
        assert_eq!(
            resolved
                .ok_or("Expected a value in test_register_fallback")?
                .name(),
            "MOCK"
        );

        Ok(())
    }
}
