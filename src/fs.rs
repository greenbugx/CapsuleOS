//! Virtual filesystem for Capsule OS.
//! This module maps in-OS paths to a sandboxed host folder under `runtime/`.

use std::cmp::Ordering;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FsEntry {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct VirtualFs {
    host_root: PathBuf,
    cwd: Vec<String>,
}

impl VirtualFs {
    pub fn new(host_root: impl Into<PathBuf>) -> Result<Self, String> {
        let host_root = host_root.into();
        fs::create_dir_all(host_root.join("home"))
            .map_err(|err| format!("Failed to initialize runtime/home: {err}"))?;

        Ok(Self {
            host_root,
            cwd: vec!["home".to_string()],
        })
    }

    pub fn host_root(&self) -> &Path {
        &self.host_root
    }

    pub fn cwd(&self) -> String {
        Self::segments_to_virtual_path(&self.cwd)
    }

    pub fn prompt_path(&self) -> String {
        if self.cwd.is_empty() {
            return "/".to_string();
        }

        if self.cwd.first().map(|part| part.as_str()) == Some("home") {
            if self.cwd.len() == 1 {
                "~".to_string()
            } else {
                format!("~/{}", self.cwd[1..].join("/"))
            }
        } else {
            self.cwd()
        }
    }

    pub fn ls(&self, path: Option<&str>) -> Result<Vec<FsEntry>, String> {
        let target = match path {
            Some(raw) => self.resolve_segments(raw)?,
            None => self.cwd.clone(),
        };
        let host_path = self.segments_to_host_path(&target);

        if !host_path.exists() {
            return Err(format!(
                "No such file or directory: {}",
                Self::segments_to_virtual_path(&target)
            ));
        }

        if host_path.is_file() {
            let name = host_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("file")
                .to_string();
            return Ok(vec![FsEntry {
                name,
                is_dir: false,
            }]);
        }

        let mut entries = Vec::new();
        for item in fs::read_dir(&host_path).map_err(|err| format!("ls failed: {err}"))? {
            let item = item.map_err(|err| format!("ls failed: {err}"))?;
            let metadata = item
                .metadata()
                .map_err(|err| format!("ls failed to read metadata: {err}"))?;
            let name = item
                .file_name()
                .to_str()
                .unwrap_or("<invalid-name>")
                .to_string();

            entries.push(FsEntry {
                name,
                is_dir: metadata.is_dir(),
            });
        }

        entries.sort_by(|left, right| match (left.is_dir, right.is_dir) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => left
                .name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.name.cmp(&right.name)),
        });

        Ok(entries)
    }

    pub fn cd(&mut self, path: &str) -> Result<(), String> {
        let target = self.resolve_segments(path)?;
        let host_path = self.segments_to_host_path(&target);

        if !host_path.exists() {
            return Err(format!(
                "No such directory: {}",
                Self::segments_to_virtual_path(&target)
            ));
        }

        if !host_path.is_dir() {
            return Err(format!(
                "Not a directory: {}",
                Self::segments_to_virtual_path(&target)
            ));
        }

        self.cwd = target;
        Ok(())
    }

    pub fn mkdir(&self, path: &str) -> Result<(), String> {
        let target = self.resolve_segments(path)?;
        let host_path = self.segments_to_host_path(&target);

        fs::create_dir_all(&host_path).map_err(|err| format!("mkdir failed: {err}"))?;
        Ok(())
    }

    pub fn touch(&self, path: &str) -> Result<(), String> {
        let target = self.resolve_segments(path)?;
        let host_path = self.segments_to_host_path(&target);

        if host_path.exists() && host_path.is_dir() {
            return Err("touch failed: target is a directory".to_string());
        }

        if let Some(parent) = host_path.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("touch failed: {err}"))?;
        }

        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&host_path)
            .map_err(|err| format!("touch failed: {err}"))?;

        Ok(())
    }

    pub fn cat(&self, path: &str) -> Result<String, String> {
        let target = self.resolve_segments(path)?;
        let host_path = self.segments_to_host_path(&target);

        if !host_path.exists() {
            return Err(format!(
                "No such file: {}",
                Self::segments_to_virtual_path(&target)
            ));
        }

        if !host_path.is_file() {
            return Err(format!(
                "Not a file: {}",
                Self::segments_to_virtual_path(&target)
            ));
        }

        let bytes = fs::read(&host_path).map_err(|err| format!("cat failed: {err}"))?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn resolve_segments(&self, path: &str) -> Result<Vec<String>, String> {
        if path.trim().is_empty() {
            return Ok(self.cwd.clone());
        }

        let mut segments = if path.starts_with('/') {
            Vec::new()
        } else {
            self.cwd.clone()
        };

        for part in path.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    if segments.pop().is_none() {
                        return Err("Cannot traverse above /".to_string());
                    }
                }
                piece => segments.push(piece.to_string()),
            }
        }

        Ok(segments)
    }

    fn segments_to_host_path(&self, segments: &[String]) -> PathBuf {
        let mut host_path = self.host_root.clone();
        for part in segments {
            host_path.push(part);
        }
        host_path
    }

    fn segments_to_virtual_path(segments: &[String]) -> String {
        if segments.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", segments.join("/"))
        }
    }
}
