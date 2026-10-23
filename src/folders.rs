use std::env;
use std::io;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Folders {
    pub downloads: PathBuf,
    pub documents: PathBuf,
    pub pictures: PathBuf,
    pub videos: PathBuf,
    pub music: PathBuf,
    pub state: PathBuf,
}

impl Folders {
    pub fn discover() -> io::Result<Self> {
        discover()
    }

    pub fn destination(&self, category: &str) -> PathBuf {
        let (root, subfolder) = category.split_once('/').unwrap_or((category, ""));
        let base = match root {
            "Pictures" => &self.pictures,
            "Videos" => &self.videos,
            "Music" => &self.music,
            "Documents" | "Archives" | "Programs" => &self.documents,
            _ => &self.documents,
        };
        if subfolder.is_empty() {
            base.join(root)
        } else {
            base.join(subfolder)
        }
    }
}

#[cfg(unix)]
fn discover() -> io::Result<Folders> {
    use std::process::Command;
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
    let user_dir = |name: &str, fallback: &str| {
        Command::new("xdg-user-dir")
            .arg(name)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|value| PathBuf::from(value.trim()))
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(fallback))
    };
    let state = env::var_os("XDG_STATE_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/state"))
        .join("getclean");
    Ok(Folders {
        downloads: user_dir("DOWNLOAD", "Downloads"),
        documents: user_dir("DOCUMENTS", "Documents"),
        pictures: user_dir("PICTURES", "Pictures"),
        videos: user_dir("VIDEOS", "Videos"),
        music: user_dir("MUSIC", "Music"),
        state,
    })
}

#[cfg(windows)]
fn discover() -> io::Result<Folders> {
    use std::ffi::{c_void, OsString};
    use std::os::windows::ffi::OsStringExt;
    use std::ptr;

    #[repr(C)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[link(name = "shell32")]
    extern "system" {
        fn SHGetKnownFolderPath(
            id: *const Guid,
            flags: u32,
            token: *mut c_void,
            path: *mut *mut u16,
        ) -> i32;
    }
    #[link(name = "ole32")]
    extern "system" {
        fn CoTaskMemFree(value: *mut c_void);
    }

    fn known(id: Guid) -> io::Result<PathBuf> {
        let mut raw = ptr::null_mut();
        let status = unsafe { SHGetKnownFolderPath(&id, 0, ptr::null_mut(), &mut raw) };
        if status != 0 || raw.is_null() {
            if !raw.is_null() {
                unsafe { CoTaskMemFree(raw.cast()) };
            }
            return Err(io::Error::other(format!(
                "SHGetKnownFolderPath failed: 0x{:08x}",
                status as u32
            )));
        }
        let mut len = 0;
        unsafe {
            while *raw.add(len) != 0 {
                len += 1;
            }
            let path = PathBuf::from(OsString::from_wide(std::slice::from_raw_parts(raw, len)));
            CoTaskMemFree(raw.cast());
            Ok(path)
        }
    }

    let home = env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "USERPROFILE is not set"))?;
    let folder = |id, fallback: &str| known(id).unwrap_or_else(|_| home.join(fallback));

    Ok(Folders {
        downloads: folder(
            Guid {
                data1: 0x374de290,
                data2: 0x123f,
                data3: 0x4565,
                data4: [0x91, 0x64, 0x39, 0xc4, 0x92, 0x5e, 0x46, 0x7b],
            },
            "Downloads",
        ),
        documents: folder(
            Guid {
                data1: 0xfdd39ad0,
                data2: 0x238f,
                data3: 0x46af,
                data4: [0xad, 0xb4, 0x6c, 0x85, 0x48, 0x03, 0x69, 0xc7],
            },
            "Documents",
        ),
        pictures: folder(
            Guid {
                data1: 0x33e28130,
                data2: 0x4e1e,
                data3: 0x4676,
                data4: [0x83, 0x5a, 0x98, 0x39, 0x5c, 0x3b, 0xc3, 0xbb],
            },
            "Pictures",
        ),
        videos: folder(
            Guid {
                data1: 0x18989b1d,
                data2: 0x99b5,
                data3: 0x455b,
                data4: [0x84, 0x1c, 0xab, 0x7c, 0x74, 0xe4, 0xdf, 0xfc],
            },
            "Videos",
        ),
        music: folder(
            Guid {
                data1: 0x4bd8d571,
                data2: 0x6d19,
                data3: 0x48d3,
                data4: [0xbe, 0x97, 0x42, 0x22, 0x20, 0x08, 0x0e, 0x43],
            },
            "Music",
        ),
        state: known(Guid {
            data1: 0xf1b32785,
            data2: 0x6fba,
            data3: 0x4fcf,
            data4: [0x9d, 0x55, 0x7b, 0x8e, 0x7f, 0x15, 0x70, 0x91],
        })
        .unwrap_or_else(|_| {
            env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join("AppData/Local"))
        })
        .join("getClean"),
    })
}
