use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{SystemTime, Duration};

use super::quality::TranscodeQuality;
use super::detector;

/// Cache manager for transcoded media files
pub struct TranscodeCache {
    cache_dir: PathBuf,
}

impl TranscodeCache {
    /// Create a new cache manager
    pub fn new(app_data_dir: &Path) -> Self {
        let cache_dir = app_data_dir.join("transcoded");
        if let Err(e) = fs::create_dir_all(&cache_dir) {
            eprintln!("WARN: Failed to create transcoding cache dir: {}", e);
        }
        Self { cache_dir }
    }

    /// Generate a deterministic cache key from source path and quality
    fn generate_cache_key(source: &Path, quality: TranscodeQuality) -> String {
        let mut hasher = DefaultHasher::new();
        source.to_string_lossy().hash(&mut hasher);
        (quality as u8).hash(&mut hasher);
        
        // Also hash the file modification time for cache invalidation
        if let Ok(metadata) = fs::metadata(source) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                    duration.as_secs().hash(&mut hasher);
                }
            }
        }
        
        format!("{:016x}", hasher.finish())
    }

    /// Get the cache file path for a source file
    pub fn get_cache_path(&self, source: &Path, quality: TranscodeQuality) -> PathBuf {
        let key = Self::generate_cache_key(source, quality);
        let ext = detector::get_output_extension(source);
        self.cache_dir.join(format!("{}.{}", key, ext))
    }

    /// Check if a cached version exists
    pub fn exists(&self, source: &Path, quality: TranscodeQuality) -> bool {
        let cache_path = self.get_cache_path(source, quality);
        cache_path.exists() && cache_path.is_file()
    }

    /// Get cached file if it exists, otherwise return None
    pub fn get(&self, source: &Path, quality: TranscodeQuality) -> Option<PathBuf> {
        let cache_path = self.get_cache_path(source, quality);
        if cache_path.exists() && cache_path.is_file() {
            // Verify the cached file is not empty or corrupted
            if let Ok(metadata) = fs::metadata(&cache_path) {
                if metadata.len() > 1024 { // At least 1KB to be valid
                    return Some(cache_path);
                }
            }
        }
        None
    }

    /// Clean up old cache entries
    /// Returns number of files deleted
    pub fn cleanup(&self, max_age_days: u64) -> usize {
        let max_age = Duration::from_secs(max_age_days * 24 * 60 * 60);
        let now = SystemTime::now();
        let mut deleted = 0;

        let entries = match fs::read_dir(&self.cache_dir) {
            Ok(e) => e,
            Err(_) => return 0,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let metadata = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let modified = match metadata.modified() {
                Ok(m) => m,
                Err(_) => continue,
            };

            if let Ok(age) = now.duration_since(modified) {
                if age > max_age {
                    if fs::remove_file(&path).is_ok() {
                        deleted += 1;
                    }
                }
            }
        }

        deleted
    }

    /// Get total cache size in bytes
    pub fn get_cache_size(&self) -> u64 {
        let entries = match fs::read_dir(&self.cache_dir) {
            Ok(e) => e,
            Err(_) => return 0,
        };

        entries.flatten()
            .filter_map(|entry| fs::metadata(entry.path()).ok())
            .filter(|m| m.is_file())
            .map(|m| m.len())
            .sum()
    }

    /// Get cache directory path
    pub fn dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Delete a specific cache entry
    pub fn invalidate(&self, source: &Path, quality: TranscodeQuality) -> bool {
        let cache_path = self.get_cache_path(source, quality);
        if cache_path.exists() {
            fs::remove_file(&cache_path).is_ok()
        } else {
            true
        }
    }

    /// Delete all cache entries for a source file (all qualities)
    pub fn invalidate_all(&self, source: &Path) -> usize {
        let mut deleted = 0;
        for quality in TranscodeQuality::all() {
            if self.invalidate(source, *quality) {
                deleted += 1;
            }
        }
        deleted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_cache_key_generation() {
        let key1 = TranscodeCache::generate_cache_key(
            Path::new("/test/file.mkv"),
            TranscodeQuality::Preview
        );
        let key2 = TranscodeCache::generate_cache_key(
            Path::new("/test/file.mkv"),
            TranscodeQuality::High
        );
        
        // Different qualities should produce different keys
        assert_ne!(key1, key2);
        
        // Same input should produce same key
        let key3 = TranscodeCache::generate_cache_key(
            Path::new("/test/file.mkv"),
            TranscodeQuality::Preview
        );
        assert_eq!(key1, key3);
    }

    #[test]
    fn test_cache_paths() {
        let temp_dir = env::temp_dir().join("mundam_cache_test");
        let cache = TranscodeCache::new(&temp_dir);
        
        let audio_path = cache.get_cache_path(
            Path::new("test.ogg"),
            TranscodeQuality::Standard
        );
        assert!(audio_path.extension().unwrap() == "m4a");
        
        let video_path = cache.get_cache_path(
            Path::new("test.mkv"),
            TranscodeQuality::High
        );
        assert!(video_path.extension().unwrap() == "mp4");
        
        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
