//! User-owned signature assets shared by document windows, independent of window snapshots.
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::fill_sign::SavedSig;

const LIMIT: u64 = 16 * 1024 * 1024;

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Assets {
    signature: Option<SavedSig>,
    initials: Option<SavedSig>,
}

pub(crate) struct Store(PathBuf);

impl Store {
    fn read(&self) -> Result<Option<Assets>, String> {
        let file = match std::fs::File::open(self.0.join("signing-assets.json")) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let mut bytes = Vec::new();
        file.take(LIMIT + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > LIMIT {
            return Err("Saved signature assets exceed 16 MiB.".into());
        }
        serde_json::from_slice(&bytes).map(Some).map_err(|e| format!("Could not read saved signature assets: {e}"))
    }

    fn lock(&self) -> Result<std::fs::File, String> {
        std::fs::create_dir_all(&self.0).map_err(|e| e.to_string())?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.0.join("signing-assets.lock"))
            .map_err(|e| e.to_string())?;
        file.try_lock().map_err(|_| "Another window is saving signature assets. Try again.".to_string())?;
        Ok(file)
    }

    fn write(&self, assets: &Assets) -> Result<(), String> {
        let bytes = serde_json::to_vec(assets).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > LIMIT {
            return Err("Saved signature assets exceed 16 MiB.".into());
        }
        let path = self.0.join("signing-assets.json");
        let path = path.to_str().ok_or("The signature settings path is not UTF-8.")?;
        crate::editing::write_atomically(path, &bytes).map_err(|e| format!("Could not save signature assets: {e}"))
    }
}

impl crate::PdfCraftApp {
    /// Connect every native document window to the same user asset file. Legacy window
    /// settings seed it once; a stale window can never replace newer assets on exit.
    pub fn connect_signing_assets(&mut self, directory: &Path) -> Result<(), String> {
        let store = Store(directory.to_path_buf());
        // Keep the store connected even on error: later edits must report the storage
        // failure, rather than silently falling back to an unsaved window snapshot.
        self.signing_assets = Some(store);
        let Some(store) = &self.signing_assets else { return Err("Signature asset storage is unavailable.".into()) };
        let _lock = store.lock()?;
        let assets = match store.read()? {
            Some(assets) => assets,
            None => {
                let assets = Assets { signature: self.signature.clone(), initials: self.initials.clone() };
                store.write(&assets)?;
                assets
            }
        };
        self.signature = assets.signature;
        self.initials = assets.initials;
        Ok(())
    }

    pub(crate) fn refresh_signing_assets(&mut self) -> Result<(), String> {
        if let Some(store) = &self.signing_assets {
            let assets = store.read()?.ok_or("The saved signature assets file is missing.")?;
            self.signature = assets.signature;
            self.initials = assets.initials;
        }
        Ok(())
    }

    /// Persist only the changed asset, merging with the latest value of the other asset.
    pub fn update_saved_signature(&mut self, initials: bool, saved: Option<SavedSig>) -> Result<(), String> {
        if let Some(store) = &self.signing_assets {
            let _lock = store.lock()?;
            let mut assets = store.read()?.ok_or("The saved signature assets file is missing.")?;
            if initials {
                assets.initials = saved.clone();
            } else {
                assets.signature = saved.clone();
            }
            store.write(&assets)?;
            self.signature = assets.signature;
            self.initials = assets.initials;
        } else if initials {
            self.initials = saved;
        } else {
            self.signature = saved;
        }
        Ok(())
    }
}
