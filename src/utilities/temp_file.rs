use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) struct TempFile {
    path: PathBuf,
    delete_on_drop: bool,
}

impl TempFile {
    pub(crate) fn new(output_path: &Path) -> io::Result<Self> {
        let output_folder = output_path.parent().unwrap_or_else(|| Path::new("."));
        let output_name = output_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("ion");
        let process_id = std::process::id();
        let time_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|time| time.as_nanos())
            .unwrap_or(0);
        let temp_name = format!(".{output_name}.tmp.{process_id}.{time_id}");
        Ok(Self {
            path: output_folder.join(temp_name),
            delete_on_drop: true,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn move_to(mut self, output_path: &Path) -> io::Result<()> {
        fs::rename(&self.path, output_path)?;
        self.delete_on_drop = false;
        Ok(())
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if self.delete_on_drop {
            let _ = fs::remove_file(&self.path);
        }
    }
}
