// Project archives (.mamba): one file holding a whole project (original
// binary, metadata, versions, map edits) to back it up, move it to another
// computer or hand it to someone. The same code duplicates a project inside
// the library.
//
// Format: gzip-compressed JSON
//   { "format": "mamba-project", "version": 1, "app_version": "1.0.0",
//     "files": { "<file name>": "<base64>", ... } }
// holding the files of one project folder (layout in src/lib/local/store.ts).
//
// An archive is never trusted: file names are restricted to plain names, and
// every identifier is regenerated on import. The project gets a new id and
// each version a new one too, because the store looks version ids up across
// the whole library: a duplicate must never reuse them.

use base64::Engine;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;

pub const FORMAT: &str = "mamba-project";
pub const FORMAT_VERSION: u64 = 1;
pub const EXTENSION: &str = "mamba";
/// Ceiling on the decompressed archive, so a crafted file cannot fill the memory.
const MAX_ARCHIVE_BYTES: u64 = 1 << 30;

/// Project ids are short alphanumeric strings (see `newId` in store.ts):
/// anything else could escape the projects directory.
pub fn is_valid_project_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric())
}

fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// 15-char lowercase alphanumeric id, same shape as `newId()` in store.ts.
pub fn new_id() -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    // Two v4 UUIDs minus their fixed version / variant bytes: 28 random bytes.
    let mut random = Vec::with_capacity(32);
    for _ in 0..2 {
        let bytes = uuid::Uuid::new_v4().into_bytes();
        random.extend(bytes.iter().enumerate().filter(|(i, _)| *i != 6 && *i != 8).map(|(_, b)| *b));
    }
    random.iter().take(15).map(|b| CHARS[*b as usize % CHARS.len()] as char).collect()
}

/// Current time as an ISO 8601 UTC string ("2026-10-29T18:04:05.000Z"),
/// the format the store writes.
fn iso_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (H. Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year,
        month,
        day,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60,
        now.subsec_millis()
    )
}

/// The files of a project folder, by name. Temporary files of an
/// interrupted write are left out.
fn read_project_files(project_dir: &Path) -> Result<HashMap<String, Vec<u8>>, String> {
    let entries = std::fs::read_dir(project_dir).map_err(|e| format!("Project folder unreadable: {e}"))?;
    let mut files = HashMap::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        if !path.is_file() || !is_safe_name(&name) || name.ends_with(".tmp") {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("Cannot read {name}: {e}"))?;
        files.insert(name, bytes);
    }
    if !files.contains_key("project.json") || !files.contains_key("original.bin") {
        return Err("Not a Mamba project folder".to_string());
    }
    Ok(files)
}

/// Pack a project folder into a `.mamba` archive.
pub fn pack_project(project_dir: &Path) -> Result<Vec<u8>, String> {
    let files = read_project_files(project_dir)?;
    let b64 = base64::engine::general_purpose::STANDARD;
    let mut names: Vec<&String> = files.keys().collect();
    names.sort();
    let mut entries = Map::new();
    for name in names {
        entries.insert(name.clone(), Value::String(b64.encode(&files[name])));
    }
    let doc = json!({
        "format": FORMAT,
        "version": FORMAT_VERSION,
        "app_version": env!("CARGO_PKG_VERSION"),
        "files": entries,
    });
    let raw = serde_json::to_vec(&doc).map_err(|e| e.to_string())?;
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&raw).map_err(|e| e.to_string())?;
    gz.finish().map_err(|e| e.to_string())
}

/// Unpack a `.mamba` archive into the projects directory as a NEW project.
/// Returns the new project id. `project_name` renames it when given.
pub fn unpack_project(projects_dir: &Path, archive: &[u8], project_name: Option<&str>) -> Result<String, String> {
    let mut raw = Vec::new();
    GzDecoder::new(archive)
        .take(MAX_ARCHIVE_BYTES)
        .read_to_end(&mut raw)
        .map_err(|_| "Not a Mamba project archive".to_string())?;
    let doc: Value = serde_json::from_slice(&raw).map_err(|_| "Not a Mamba project archive".to_string())?;
    if doc["format"].as_str() != Some(FORMAT) {
        return Err("Not a Mamba project archive".to_string());
    }
    if doc["version"].as_u64().unwrap_or(0) > FORMAT_VERSION {
        return Err("This project archive comes from a newer Mamba: update Mamba to open it".to_string());
    }
    let entries = doc["files"].as_object().ok_or("Archive without files")?;
    let b64 = base64::engine::general_purpose::STANDARD;
    let mut files = HashMap::new();
    for (name, content) in entries {
        if !is_safe_name(name) {
            return Err(format!("Invalid file name in archive: {name}"));
        }
        let bytes = content
            .as_str()
            .and_then(|s| b64.decode(s).ok())
            .ok_or_else(|| format!("Corrupted file in archive: {name}"))?;
        files.insert(name.clone(), bytes);
    }
    if !files.contains_key("project.json") || !files.contains_key("original.bin") {
        return Err("The archive holds no project".to_string());
    }
    write_as_new_project(projects_dir, files, project_name)
}

/// Copy a project of the library as a new project. Returns the new id.
pub fn duplicate_project(projects_dir: &Path, file_id: &str, project_name: Option<&str>) -> Result<String, String> {
    if !is_valid_project_id(file_id) {
        return Err("Invalid project id".to_string());
    }
    let files = read_project_files(&projects_dir.join(file_id))?;
    write_as_new_project(projects_dir, files, project_name)
}

/// Version id carried by a per-version file name, and the name rebuilt
/// around another id: edits-<id>.json, version-<id>.bin, version-<id>-maps.json.
fn version_file(name: &str) -> Option<(&str, Box<dyn Fn(&str) -> String>)> {
    if let Some(id) = name.strip_prefix("edits-").and_then(|r| r.strip_suffix(".json")) {
        return Some((id, Box::new(|v| format!("edits-{v}.json"))));
    }
    if let Some(id) = name.strip_prefix("version-").and_then(|r| r.strip_suffix("-maps.json")) {
        return Some((id, Box::new(|v| format!("version-{v}-maps.json"))));
    }
    if let Some(id) = name.strip_prefix("version-").and_then(|r| r.strip_suffix(".bin")) {
        return Some((id, Box::new(|v| format!("version-{v}.bin"))));
    }
    None
}

fn write_as_new_project(
    projects_dir: &Path,
    mut files: HashMap<String, Vec<u8>>,
    project_name: Option<&str>,
) -> Result<String, String> {
    std::fs::create_dir_all(projects_dir).map_err(|e| e.to_string())?;
    let fresh_id = || loop {
        let id = new_id();
        if !projects_dir.join(&id).exists() {
            break id;
        }
    };
    let project_id = fresh_id();
    let now = iso_now();

    // Versions: new ids, owned by the new project
    let mut versions: Vec<Value> = match files.remove("versions.json") {
        Some(bytes) => serde_json::from_slice(&bytes).map_err(|_| "Corrupted versions.json".to_string())?,
        None => Vec::new(),
    };
    let mut version_ids: HashMap<String, String> = HashMap::new();
    for v in &versions {
        if let Some(id) = v["id"].as_str() {
            version_ids.insert(id.to_string(), new_id());
        }
    }
    for v in versions.iter_mut() {
        let Some(obj) = v.as_object_mut() else { continue };
        if let Some(new) = obj.get("id").and_then(Value::as_str).and_then(|id| version_ids.get(id)) {
            obj.insert("id".into(), Value::String(new.clone()));
        }
        obj.insert("file".into(), Value::String(project_id.clone()));
        let base = obj.get("base_version").and_then(Value::as_str).and_then(|b| version_ids.get(b)).cloned();
        obj.insert("base_version".into(), base.map(Value::String).unwrap_or(Value::Null));
    }

    // Project record: new id, optional new name, created now
    let mut project: Value = serde_json::from_slice(&files.remove("project.json").unwrap_or_default())
        .map_err(|_| "Corrupted project.json".to_string())?;
    let obj = project.as_object_mut().ok_or("Corrupted project.json")?;
    obj.insert("id".into(), Value::String(project_id.clone()));
    obj.insert("created".into(), Value::String(now.clone()));
    obj.insert("updated".into(), Value::String(now));
    if let Some(name) = project_name.map(str::trim).filter(|n| !n.is_empty()) {
        obj.insert("project_name".into(), Value::String(name.to_string()));
    }

    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, bytes) in files {
        if let Some((old_id, rename)) = version_file(&name) {
            // A file of a version the project does not list is unreachable: dropped
            let Some(new) = version_ids.get(old_id) else { continue };
            let content = if name.starts_with("edits-") {
                let mut edits: Vec<Value> =
                    serde_json::from_slice(&bytes).map_err(|_| format!("Corrupted {name}"))?;
                for e in edits.iter_mut().filter_map(Value::as_object_mut) {
                    e.insert("version".into(), Value::String(new.clone()));
                }
                serde_json::to_vec(&edits).map_err(|e| e.to_string())?
            } else {
                bytes
            };
            out.push((rename(new), content));
        } else {
            out.push((name, bytes));
        }
    }
    out.push(("versions.json".into(), serde_json::to_vec_pretty(&versions).map_err(|e| e.to_string())?));
    // project.json last: until it exists the folder is invisible to the library
    out.push(("project.json".into(), serde_json::to_vec_pretty(&project).map_err(|e| e.to_string())?));

    let dir = projects_dir.join(&project_id);
    let written = (|| -> std::io::Result<()> {
        std::fs::create_dir(&dir)?;
        for (name, bytes) in &out {
            std::fs::write(dir.join(name), bytes)?;
        }
        Ok(())
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(format!("Cannot write the project: {e}"));
    }
    Ok(project_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("mamba-archive-test-{}", new_id()));
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn read_json(path: PathBuf) -> Value {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    /// A project shaped like the ones store.ts writes: Ori + one imported version.
    fn sample_project(projects: &Path) -> String {
        let id = "abcdefghij12345".to_string();
        let dir = projects.join(&id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("original.bin"), [1u8, 2, 3, 4]).unwrap();
        std::fs::write(
            dir.join("project.json"),
            serde_json::to_vec(&json!({
                "id": id, "file_name": "ori.bin", "project_name": "Golf",
                "created": "2026-01-01T00:00:00.000Z", "updated": "2026-01-01T00:00:00.000Z"
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join("versions.json"),
            serde_json::to_vec(&json!([
                {"id": "ori000000000001", "file": id, "name": "Ori", "is_current": false, "base_version": null, "created": "x"},
                {"id": "stg000000000001", "file": id, "name": "Stage 1", "is_current": true, "base_version": "ori000000000001", "created": "y"}
            ]))
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join("edits-stg000000000001.json"),
            serde_json::to_vec(&json!([{"id": "e1", "version": "stg000000000001", "map_address": 16, "payload": {}}])).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("version-stg000000000001.bin"), [9u8, 9, 9, 9]).unwrap();
        std::fs::write(dir.join("version-orphan000000001.bin"), [7u8]).unwrap();
        std::fs::write(dir.join("project.json.abc.tmp"), b"half written").unwrap();
        id
    }

    fn check_copy(projects: &Path, old_id: &str, new_id: &str) {
        assert_ne!(old_id, new_id);
        let dir = projects.join(new_id);
        assert_eq!(std::fs::read(dir.join("original.bin")).unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(read_json(dir.join("project.json"))["id"], new_id);

        let versions = read_json(dir.join("versions.json"));
        let versions = versions.as_array().unwrap();
        assert_eq!(versions.len(), 2);
        let ori = versions[0]["id"].as_str().unwrap();
        let stage = versions[1]["id"].as_str().unwrap();
        assert_ne!(ori, "ori000000000001");
        assert_ne!(stage, "stg000000000001");
        assert_eq!(versions[1]["base_version"], ori);
        assert!(versions.iter().all(|v| v["file"] == new_id));

        let edits = read_json(dir.join(format!("edits-{stage}.json")));
        assert_eq!(edits[0]["version"], stage);
        assert_eq!(edits[0]["map_address"], 16);
        assert_eq!(std::fs::read(dir.join(format!("version-{stage}.bin"))).unwrap(), vec![9, 9, 9, 9]);
        // orphan and temporary files are not carried over
        let names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(names.len(), 5, "{names:?}");
    }

    #[test]
    fn archive_round_trip_creates_a_new_project() {
        let tmp = TempDir::new();
        let old_id = sample_project(&tmp.0);
        let archive = pack_project(&tmp.0.join(&old_id)).unwrap();
        let new_id = unpack_project(&tmp.0, &archive, None).unwrap();
        check_copy(&tmp.0, &old_id, &new_id);
        assert_eq!(read_json(tmp.0.join(&new_id).join("project.json"))["project_name"], "Golf");
        // the source project is untouched
        assert_eq!(read_json(tmp.0.join(&old_id).join("project.json"))["id"], old_id.as_str());
    }

    #[test]
    fn duplicate_renames_the_copy() {
        let tmp = TempDir::new();
        let old_id = sample_project(&tmp.0);
        let new_id = duplicate_project(&tmp.0, &old_id, Some("Golf (copy)")).unwrap();
        check_copy(&tmp.0, &old_id, &new_id);
        let project = read_json(tmp.0.join(&new_id).join("project.json"));
        assert_eq!(project["project_name"], "Golf (copy)");
        assert_ne!(project["created"], "2026-01-01T00:00:00.000Z");
    }

    #[test]
    fn hostile_or_foreign_archives_are_refused() {
        let tmp = TempDir::new();
        let gz = |doc: Value| {
            let mut e = GzEncoder::new(Vec::new(), Compression::default());
            e.write_all(&serde_json::to_vec(&doc).unwrap()).unwrap();
            e.finish().unwrap()
        };
        let b64 = |s: &[u8]| base64::engine::general_purpose::STANDARD.encode(s);
        let escaping = gz(json!({"format": FORMAT, "version": 1, "files": {
            "project.json": b64(b"{}"), "original.bin": b64(b"x"), "../evil": b64(b"x")
        }}));
        assert!(unpack_project(&tmp.0, &escaping, None).is_err());
        let foreign = gz(json!({"format": "something-else", "files": {}}));
        assert!(unpack_project(&tmp.0, &foreign, None).is_err());
        let newer = gz(json!({"format": FORMAT, "version": 99, "files": {}}));
        assert!(unpack_project(&tmp.0, &newer, None).unwrap_err().contains("newer"));
        assert!(unpack_project(&tmp.0, b"plain bytes", None).is_err());
        assert!(duplicate_project(&tmp.0, "../x", None).is_err());
        // nothing was left behind
        assert_eq!(std::fs::read_dir(&tmp.0).unwrap().count(), 0);
    }

    #[test]
    fn ids_and_dates_have_the_store_shape() {
        let id = new_id();
        assert_eq!(id.len(), 15);
        assert!(id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
        let now = iso_now();
        assert_eq!(now.len(), 24);
        assert!(now.starts_with("20") && now.ends_with('Z'));
    }
}
