use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::{self, Metadata, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub mod folders;
pub mod history;
use folders::Folders;
use history::{modified_nanos, History};

const OLD_FILE_AGE: Duration = Duration::from_secs(30);

/// Only files directly inside Downloads are organized. Subfolders are left alone.
pub struct Organizer {
    folders: Folders,
    pending: HashMap<PathBuf, Observation>,
    ignored: HashMap<PathBuf, (u64, u128)>,
    history_stamp: Option<(u64, Option<SystemTime>)>,
    settle_time: Duration,
}

#[derive(Clone, Copy)]
struct Observation {
    len: u64,
    modified: Option<SystemTime>,
    first_seen: SystemTime,
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct ScanResult {
    pub moved: usize,
    pub waiting: usize,
    pub failed: usize,
}

impl Organizer {
    pub fn new(folders: Folders, settle_time: Duration) -> Self {
        Self {
            folders,
            pending: HashMap::new(),
            ignored: HashMap::new(),
            history_stamp: None,
            settle_time,
        }
    }

    pub fn downloads(&self) -> &Path {
        &self.folders.downloads
    }

    pub fn scan(&mut self, dry_run: bool) -> io::Result<ScanResult> {
        self.refresh_ignored()?;
        let mut result = ScanResult::default();
        let mut seen = HashSet::new();

        for entry in fs::read_dir(&self.folders.downloads)? {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    eprintln!("getClean: cannot read a Downloads entry: {err}");
                    result.failed += 1;
                    continue;
                }
            };
            let path = entry.path();
            let Some(folder) = category(&path) else {
                continue;
            };
            let meta = match fs::symlink_metadata(&path) {
                Ok(meta) if meta.file_type().is_file() => meta,
                Ok(_) => continue,
                Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => {
                    eprintln!("getClean: cannot inspect {}: {err}", path.display());
                    result.failed += 1;
                    continue;
                }
            };
            if self.ignored.get(&path).is_some_and(|&(len, modified)| {
                len == meta.len() && modified == modified_nanos(&meta)
            }) {
                continue;
            }
            self.ignored.remove(&path);
            seen.insert(path.clone());
            if !self.is_ready(&path, &meta) {
                result.waiting += 1;
                continue;
            }
            let dest_dir = self.folders.destination(folder);
            if dry_run {
                println!("{} -> {}", path.display(), dest_dir.display());
                result.moved += 1;
                continue;
            }
            let name = path.file_name().expect("directory entry has a filename");
            match move_unique_named(&path, &dest_dir, name) {
                Ok(dest) => {
                    let history = History::new(self.folders.state.join("history.log"));
                    if let Err(err) = history.record_move(&path, &dest) {
                        // A move without history cannot be undone. Try to put it back.
                        let rollback = move_unique_named(&dest, &self.folders.downloads, name);
                        eprintln!(
                            "getClean: cannot record move of {}: {err}; rollback: {rollback:?}",
                            path.display()
                        );
                        result.failed += 1;
                        continue;
                    }
                    println!("{} -> {}", path.display(), dest.display());
                    result.moved += 1;
                    self.pending.remove(&path);
                    let history_path = self.folders.state.join("history.log");
                    self.history_stamp = fs::metadata(history_path)
                        .ok()
                        .map(|meta| (meta.len(), meta.modified().ok()));
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => {
                    self.pending.remove(&path);
                }
                Err(err) => {
                    eprintln!("getClean: cannot move {}: {err}", path.display());
                    result.failed += 1;
                }
            }
        }

        self.pending.retain(|path, _| seen.contains(path));
        Ok(result)
    }

    fn refresh_ignored(&mut self) -> io::Result<()> {
        let path = self.folders.state.join("history.log");
        let stamp = fs::metadata(&path)
            .ok()
            .map(|meta| (meta.len(), meta.modified().ok()));
        if stamp != self.history_stamp {
            self.ignored = History::new(path).ignored_files()?;
            self.history_stamp = stamp;
        }
        Ok(())
    }

    fn is_ready(&mut self, path: &Path, meta: &Metadata) -> bool {
        let now = SystemTime::now();
        let modified = meta.modified().ok();
        let age = modified.and_then(|time| now.duration_since(time).ok());
        let observation = Observation {
            len: meta.len(),
            modified,
            first_seen: now,
        };

        // Existing, untouched files can be cleared immediately on startup.
        if age.is_some_and(|age| age >= OLD_FILE_AGE) {
            return true;
        }
        match self.pending.insert(path.to_path_buf(), observation) {
            Some(previous) if previous.len == observation.len && previous.modified == modified => {
                // Keep the original observation while the file remains unchanged.
                self.pending.insert(path.to_path_buf(), previous);
                now.duration_since(previous.first_seen).unwrap_or_default() >= self.settle_time
                    && age.is_some_and(|age| age >= self.settle_time)
            }
            _ => false,
        }
    }
}

pub fn category(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => Some("Pictures/jpeg"),
        "png" => Some("Pictures/png"),
        "gif" => Some("Pictures/gif"),
        "webp" => Some("Pictures/webp"),
        "svg" => Some("Pictures/svg"),
        "bmp" => Some("Pictures/bmp"),
        "ico" => Some("Pictures/ico"),
        "mp4" => Some("Videos/mp4"),
        "mkv" => Some("Videos/mkv"),
        "avi" => Some("Videos/avi"),
        "mov" => Some("Videos/mov"),
        "webm" => Some("Videos/webm"),
        "mp3" => Some("Music/mp3"),
        "wav" => Some("Music/wav"),
        "flac" => Some("Music/flac"),
        "ogg" => Some("Music/ogg"),
        "pdf" => Some("Documents/pdf"),
        "docx" => Some("Documents/word"),
        "xlsx" => Some("Documents/excel"),
        "pptx" => Some("Documents/powerpoint"),
        "txt" => Some("Documents/txt"),
        "zip" | "rar" | "7z" | "tar" | "gz" => Some("Archives"),
        "exe" | "msi" | "dmg" | "iso" => Some("Programs"),
        _ => None,
    }
}

pub(crate) fn move_unique_named(
    source: &Path,
    dest_dir: &Path,
    filename: &OsStr,
) -> io::Result<PathBuf> {
    fs::create_dir_all(dest_dir)?;
    let filename_path = Path::new(filename);
    let stem = filename_path
        .file_stem()
        .unwrap_or(filename)
        .to_string_lossy();
    let ext = filename_path.extension().map(|e| e.to_string_lossy());

    for index in 0..10_000 {
        let name = if index == 0 {
            filename.to_os_string()
        } else {
            let suffix = ext.as_ref().map(|e| format!(".{e}")).unwrap_or_default();
            format!("{stem} ({index}){suffix}").into()
        };
        let dest = dest_dir.join(name);
        // hard_link atomically claims a free name and never replaces another file.
        match fs::hard_link(source, &dest) {
            Ok(()) => {
                if let Err(err) = fs::remove_file(source) {
                    let _ = fs::remove_file(&dest);
                    return Err(err);
                }
                return Ok(dest);
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Err(err),
            Err(_) => match copy_new_and_remove(source, &dest) {
                Ok(()) => return Ok(dest),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(err),
            },
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "too many files with the same name",
    ))
}

fn copy_new_and_remove(source: &Path, dest: &Path) -> io::Result<()> {
    let before = fs::metadata(source)?;
    let mut input = fs::File::open(source)?;
    let mut output = OpenOptions::new().write(true).create_new(true).open(dest)?;
    let result = (|| {
        let copied = io::copy(&mut input, &mut output)?;
        output.sync_all()?;
        let after = fs::metadata(source)?;
        if copied != before.len()
            || after.len() != before.len()
            || after.modified().ok() != before.modified().ok()
        {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "source changed during copy",
            ));
        }
        Ok(())
    })();
    drop(input);
    drop(output);
    let result = result.and_then(|()| fs::remove_file(source));
    if result.is_err() {
        let _ = fs::remove_file(dest);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let id = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("getclean-test-{}-{id}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn folders(root: &Path) -> Folders {
        let downloads = root.join("Downloads");
        fs::create_dir_all(&downloads).unwrap();
        Folders {
            downloads,
            documents: root.join("Documents"),
            pictures: root.join("Pictures"),
            videos: root.join("Videos"),
            music: root.join("Music"),
            state: root.join("state"),
        }
    }

    #[test]
    fn categories_match_original_layout() {
        assert_eq!(category(Path::new("paper.PDF")), Some("Documents/pdf"));
        assert_eq!(category(Path::new("image.JpG")), Some("Pictures/jpeg"));
        assert_eq!(category(Path::new("download.crdownload")), None);
        let root = temp_dir();
        let folders = folders(&root);
        assert_eq!(
            folders.destination("Documents/pdf"),
            folders.documents.join("pdf")
        );
        assert_eq!(
            folders.destination("Pictures/jpeg"),
            folders.pictures.join("jpeg")
        );
        assert_eq!(
            folders.destination("Archives"),
            folders.documents.join("Archives")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sorts_and_preserves_collisions() {
        let root = temp_dir();
        let folders = folders(&root);
        fs::write(folders.downloads.join("paper.pdf"), b"one").unwrap();
        let mut organizer = Organizer::new(folders.clone(), Duration::ZERO);
        organizer.scan(false).unwrap();
        organizer.scan(false).unwrap();
        fs::write(folders.downloads.join("paper.pdf"), b"two").unwrap();
        organizer.scan(false).unwrap();
        organizer.scan(false).unwrap();
        let target = folders.documents.join("pdf");
        assert_eq!(fs::read(target.join("paper.pdf")).unwrap(), b"one");
        assert_eq!(fs::read(target.join("paper (1).pdf")).unwrap(), b"two");
        let history = History::new(folders.state.join("history.log"));
        assert_eq!(history.entries().unwrap().len(), 2);
        let (_, restored) = history.undo_latest().unwrap().unwrap();
        assert_eq!(restored, folders.downloads.join("paper.pdf"));
        assert_eq!(fs::read(&restored).unwrap(), b"two");
        assert_eq!(organizer.scan(false).unwrap().moved, 0);
        assert!(restored.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ignores_directories_and_partial_files() {
        let root = temp_dir();
        let folders = folders(&root);
        fs::create_dir(folders.downloads.join("nested.pdf")).unwrap();
        fs::write(folders.downloads.join("part.crdownload"), b"partial").unwrap();
        let mut organizer = Organizer::new(folders.clone(), Duration::ZERO);
        assert_eq!(organizer.scan(false).unwrap().moved, 0);
        assert!(!folders.documents.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn waits_until_a_download_stops_changing() {
        let root = temp_dir();
        let folders = folders(&root);
        let source = folders.downloads.join("incoming.pdf");
        fs::write(&source, b"first").unwrap();
        let mut organizer = Organizer::new(folders.clone(), Duration::from_millis(40));
        assert_eq!(organizer.scan(false).unwrap().waiting, 1);
        std::thread::sleep(Duration::from_millis(50));
        fs::write(&source, b"longer content").unwrap();
        assert_eq!(organizer.scan(false).unwrap().waiting, 1);
        assert!(!folders.documents.join("pdf/incoming.pdf").exists());
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(organizer.scan(false).unwrap().moved, 1);
        assert_eq!(
            fs::read(folders.documents.join("pdf/incoming.pdf")).unwrap(),
            b"longer content"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
