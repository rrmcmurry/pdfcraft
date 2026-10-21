use std::fs::{File, OpenOptions};
use std::path::Path;

/// Each document window holds a shared lock. Applying requires it to be the last window.
pub struct SessionLease(File);
impl SessionLease {
    /// Join the installed application's process lease. Standalone tools do not participate.
    pub fn installed() -> Result<Option<Self>, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let Some(current) = exe.parent().filter(|path| path.file_name().is_some_and(|name| name == "current")) else { return Ok(None) };
        let root = current.parent().ok_or("The installed package root is unavailable.")?;
        if !root.join("Update.exe").is_file() {
            return Ok(None);
        }
        Self::open(&root.join("UpdateSession.lock")).map(Some)
    }
    pub fn open(path: &Path) -> Result<Self, String> {
        let file = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path).map_err(|e| e.to_string())?;
        file.lock_shared().map_err(|e| e.to_string())?;
        Ok(Self(file))
    }
    pub fn exclusive(&self) -> Result<(), String> {
        self.0.unlock().map_err(|e| e.to_string())?;
        if self.0.try_lock().is_err() {
            self.0.lock_shared().map_err(|e| e.to_string())?;
            return Err("Close the other McMurryPDF windows and CLI jobs before installing this update.".into());
        }
        Ok(())
    }
    pub fn shared(&self) -> Result<(), String> {
        self.0.unlock().and_then(|()| self.0.lock_shared()).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn another_window_blocks_update_until_it_closes() {
        let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("mcmurry-update-session-{}-{unique}.lock", std::process::id()));
        let first = SessionLease::open(&path).unwrap();
        let second = SessionLease::open(&path).unwrap();
        assert!(first.exclusive().is_err());
        drop(second);
        first.exclusive().unwrap();
        first.shared().unwrap();
        let third = SessionLease::open(&path).unwrap();
        drop(third);
        drop(first);
        std::fs::remove_file(path).unwrap();
    }
}
