//! Transfer generated PDFs to a new window without turning them into saved documents.
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const LIMIT: u64 = 256 * 1024 * 1024;

fn transfer_dir() -> Result<PathBuf, String> {
    super::settings_dir().map(|dir| dir.join("WindowTransfers")).ok_or_else(|| "the private settings folder is unavailable".into())
}

pub fn launch_created(name: &str, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() as u64 > LIMIT {
        return Err("generated PDF exceeds the 256 MiB window-transfer limit".into());
    }
    let dir = transfer_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut file = tempfile::Builder::new().prefix("window-").suffix(".pdf").tempfile_in(&dir).map_err(|e| e.to_string())?;
    file.write_all(bytes).and_then(|()| file.as_file().sync_all()).map_err(|e| e.to_string())?;
    let (file_handle, path) = file.keep().map_err(|e| e.to_string())?;
    drop(file_handle);
    let result = std::env::current_exe()
        .map_err(|e| e.to_string())
        .and_then(|exe| std::process::Command::new(exe).args(["--created-name", name]).arg(&path).spawn().map(|_| ()).map_err(|e| e.to_string()));
    if result.is_err() {
        let _ = std::fs::remove_file(&path);
    }
    result
}

fn validated_transfer(path: &str) -> Result<PathBuf, String> {
    validated_in(&transfer_dir()?, path)
}

fn validated_in(root: &Path, path: &str) -> Result<PathBuf, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let file = Path::new(path).canonicalize().map_err(|e| e.to_string())?;
    if file.parent() != Some(root.as_path()) || file.extension().is_none_or(|ext| ext != "pdf") || !file.is_file() {
        return Err("the generated-document transfer must be in the private WindowTransfers folder".into());
    }
    Ok(file)
}

pub fn read_transfer(path: &str) -> Result<Vec<u8>, String> {
    let path = validated_transfer(path)?;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > LIMIT {
        return Err("generated PDF exceeds the 256 MiB window-transfer limit".into());
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("generated PDF exceeds the 256 MiB window-transfer limit".into());
    }
    Ok(bytes)
}

pub fn remove_transfer(path: &str) {
    if let Ok(path) = validated_transfer(path) {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfers_accept_only_regular_pdfs_directly_inside_the_private_folder() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("WindowTransfers");
        std::fs::create_dir(&root).unwrap();
        let valid = root.join("window.pdf");
        std::fs::write(&valid, b"%PDF-1.7").unwrap();
        assert!(validated_in(&root, valid.to_str().unwrap()).is_ok());
        let outside = base.path().join("outside.pdf");
        std::fs::write(&outside, b"%PDF-1.7").unwrap();
        assert!(validated_in(&root, outside.to_str().unwrap()).is_err());
        let nested = root.join("nested");
        std::fs::create_dir(&nested).unwrap();
        let child = nested.join("child.pdf");
        std::fs::write(&child, b"%PDF-1.7").unwrap();
        assert!(validated_in(&root, child.to_str().unwrap()).is_err());
        let directory = root.join("directory.pdf");
        std::fs::create_dir(&directory).unwrap();
        assert!(validated_in(&root, directory.to_str().unwrap()).is_err());
        let wrong_type = root.join("file.txt");
        std::fs::write(&wrong_type, b"text").unwrap();
        assert!(validated_in(&root, wrong_type.to_str().unwrap()).is_err());
    }
}
