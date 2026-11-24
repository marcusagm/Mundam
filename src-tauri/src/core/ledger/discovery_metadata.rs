//! Pure metadata policies shared by catalog mutations and future import entry points.
use crate::core::error::{AppError, AppResult};

/// Keeps the first spelling and order of nonempty, comma-separated keyword labels.
pub fn normalize_keywords(values: &[String]) -> Vec<String> {
    let mut keywords: Vec<String> = Vec::new();
    for value in values.iter().flat_map(|value| value.split(',')) {
        let keyword = value.trim();
        if keyword.is_empty() {
            continue;
        }
        if !keywords
            .iter()
            .any(|existing| existing.to_lowercase() == keyword.to_lowercase())
        {
            keywords.push(keyword.to_owned());
        }
    }
    keywords
}

/// Preserves a browser page URL, treating an empty edit as removal.
///
/// # Errors
/// Returns `ValidationFailed` for malformed URLs, non-HTTP(S) schemes or credentials.
pub fn normalize_source_url(value: &str) -> AppResult<Option<String>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let parsed = url::Url::parse(value)
        .map_err(|_| AppError::ValidationFailed("Enter a valid HTTP or HTTPS URL".into()))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(AppError::ValidationFailed(
            "Enter an HTTP or HTTPS URL without credentials".into(),
        ));
    }
    Ok(Some(value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_normalization_preserves_phrases_and_first_spelling() {
        assert_eq!(
            normalize_keywords(&[
                " Nature, nature, concept art, ".into(),
                "concept art".into()
            ]),
            vec!["Nature", "concept art"]
        );
    }

    #[test]
    fn origin_policy_rejects_unsafe_schemes_and_credentials() -> AppResult<()> {
        for value in [
            "javascript:alert(1)",
            "file:///tmp/source",
            "data:text/plain,example",
            "invalid",
            "https://user:secret@example.com",
        ] {
            assert!(normalize_source_url(value).is_err(), "accepted {value}");
        }
        assert_eq!(normalize_source_url("  ")?, None);
        assert_eq!(
            normalize_source_url(" https://example.com/page?q=1#section ")?,
            Some("https://example.com/page?q=1#section".into())
        );
        Ok(())
    }
}
