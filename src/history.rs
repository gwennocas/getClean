use crate::move_unique_named;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: u128,
    pub timestamp: u64,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub restored: Option<Restored>,
}

#[derive(Clone, Debug)]
pub struct Restored {
    pub path: PathBuf,
    pub len: u64,
    pub modified: u128,
}

pub struct History {
    path: PathBuf,
}

impl History {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn entries(&self) -> io::Result<Vec<Entry>> {
        let content = match fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err),
        };
        let mut entries = Vec::new();
        for line in content.lines() {
            let fields: Vec<_> = line.split('\t').collect();
            match fields.as_slice() {
                ["M", id, timestamp, source, destination] => {
                    if let (Ok(id), Ok(timestamp), Some(source), Some(destination)) = (
                        id.parse(),
                        timestamp.parse(),
                        decode_path(source),
                        decode_path(destination),
                    ) {
                        entries.push(Entry {
                            id,
                            timestamp,
                            source,
                            destination,
                            restored: None,
                        });
                    }
                }
                ["U", id, restored, len, modified] => {
                    if let (Ok(id), Some(path), Ok(len), Ok(modified)) = (
                        id.parse::<u128>(),
                        decode_path(restored),
                        len.parse(),
                        modified.parse(),
                    ) {
                        if let Some(entry) = entries.iter_mut().rev().find(|entry| entry.id == id) {
                            entry.restored = Some(Restored {
                                path,
                                len,
                                modified,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(entries)
    }

    pub fn ignored_files(&self) -> io::Result<HashMap<PathBuf, (u64, u128)>> {
        let content = match fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(HashMap::new()),
            Err(err) => return Err(err),
        };
        let mut ignored = HashMap::new();
        for line in content.lines() {
            let fields: Vec<_> = line.split('\t').collect();
            if let ["U", _, path, len, modified] = fields.as_slice() {
                if let (Some(path), Ok(len), Ok(modified)) =
                    (decode_path(path), len.parse(), modified.parse())
                {
                    ignored.insert(path, (len, modified));
                }
            }
        }
        Ok(ignored)
    }

    pub fn record_move(&self, source: &Path, destination: &Path) -> io::Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let id =
            (now.as_nanos() << 16) | (NEXT_ID.fetch_add(1, Ordering::Relaxed) as u128 & 0xffff);
        self.append(&format!(
            "M\t{id}\t{}\t{}\t{}\n",
            now.as_secs(),
            encode_path(source),
            encode_path(destination)
        ))
    }

    pub fn undo_latest(&self) -> io::Result<Option<(PathBuf, PathBuf)>> {
        let entry = match self
            .entries()?
            .into_iter()
            .rev()
            .find(|entry| entry.restored.is_none())
        {
            Some(entry) => entry,
            None => return Ok(None),
        };
        let parent = entry
            .source
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid history source"))?;
        let name = entry.source.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid history filename")
        })?;
        let restored = move_unique_named(&entry.destination, parent, name)?;
        let meta = fs::metadata(&restored)?;
        let modified = modified_nanos(&meta);
        if let Err(err) = self.append(&format!(
            "U\t{}\t{}\t{}\t{}\n",
            entry.id,
            encode_path(&restored),
            meta.len(),
            modified
        )) {
            // Keep the journal and the file in agreement if the undo cannot be recorded.
            let original_parent = entry.destination.parent().unwrap();
            let original_name = entry.destination.file_name().unwrap();
            let _ = move_unique_named(&restored, original_parent, original_name);
            return Err(err);
        }
        Ok(Some((entry.destination, restored)))
    }

    fn append(&self, record: &str) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(record.as_bytes())?;
        file.sync_data()
    }
}

pub fn modified_nanos(meta: &fs::Metadata) -> u128 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn encode_path(path: &Path) -> String {
    let bytes = path_bytes(path);
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn decode_path(hex: &str) -> Option<PathBuf> {
    if hex.len() % 2 != 0 {
        return None;
    }
    let bytes = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    path_from_bytes(bytes)
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(unix)]
fn path_from_bytes(bytes: Vec<u8>) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    Some(OsString::from_vec(bytes).into())
}

#[cfg(windows)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}

#[cfg(windows)]
fn path_from_bytes(bytes: Vec<u8>) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    if bytes.len() % 2 != 0 {
        return None;
    }
    let wide: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    Some(OsString::from_wide(&wide).into())
}
