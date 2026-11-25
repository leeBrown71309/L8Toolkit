//! Small helpers to persist JSON documents safely.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

/// Reads a JSON document, falling back to `T::default()` when the file is
/// missing. A corrupt file is kept aside (`*.corrupt`) instead of being
/// silently overwritten, so user data is never lost without a trace.
pub fn read_or_default<T: DeserializeOwned + Default>(path: &Path) -> T {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == ErrorKind::NotFound => return T::default(),
        Err(error) => {
            log::warn!("Cannot read {}: {error}. Using defaults.", path.display());
            return T::default();
        }
    };

    match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            log::warn!("{} is not valid JSON ({error}). Using defaults.", path.display());
            let backup = path.with_extension("corrupt.json");
            if let Err(copy_error) = fs::copy(path, &backup) {
                log::warn!("Cannot back up {}: {copy_error}", path.display());
            }
            T::default()
        }
    }
}

/// Writes through a temporary file then renames it, so a crash mid-write
/// never leaves a truncated document behind.
pub fn write_atomically<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create {}: {error}", parent.display()))?;
    }
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Cannot serialise {}: {error}", path.display()))?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, payload)
        .map_err(|error| format!("Cannot write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("Cannot replace {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
    struct Sample {
        value: u32,
    }

    #[test]
    fn missing_file_yields_default() {
        let path = std::env::temp_dir().join("l8-json-file-missing.json");
        let _ = fs::remove_file(&path);
        assert_eq!(read_or_default::<Sample>(&path), Sample::default());
    }

    #[test]
    fn round_trips_and_recovers_from_corruption() {
        let dir = std::env::temp_dir().join("l8-json-file-test");
        let path = dir.join("sample.json");
        write_atomically(&path, &Sample { value: 7 }).unwrap();
        assert_eq!(read_or_default::<Sample>(&path), Sample { value: 7 });

        fs::write(&path, "{ not json").unwrap();
        assert_eq!(read_or_default::<Sample>(&path), Sample::default());
        assert!(dir.join("sample.corrupt.json").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
