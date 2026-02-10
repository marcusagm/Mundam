use std::collections::HashMap;
use std::path::Path;

pub fn read_exif(path: &Path) -> HashMap<String, String> {
    let mut result = HashMap::new();

    // rexif parses directly from file path
    if let Ok(data) = rexif::parse_file(path.to_string_lossy().as_ref()) {
        for entry in data.entries {
            // entry.tag is an enum, we convert it to string
            let key = entry.tag.to_string();
            // entry.value is an enum (Ascii, Short, etc), formatted by DISPLAY trait
            let value = entry.value_more_readable.to_string();

            if !value.trim().is_empty() {
                result.insert(key, value);
            }
        }
    }

    result
}
