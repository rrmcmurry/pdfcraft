//! Installed Windows lifecycle. The updater never receives document paths or contents.
use pdfcraft_platform::update_session::SessionLease;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use pdfcraft_ui_egui::updates::{InstalledAction, InstalledPhase, InstalledRelease};
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, UpdateOptions, sources};
use winreg::{RegKey, enums::*};

const REPOSITORY: &str = "https://github.com/rrmcmurry/pdfcraft";
const PROGID: &str = "Applications\\McMurryPDF.exe";
const THUMBNAIL_CLSID: &str = "{8BE52C39-5E5E-4D32-A663-D1BEB26F65AD}";
static FIRST_RUN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn startup() {
    velopack::VelopackApp::build()
        .set_auto_apply_on_startup(false)
        .on_first_run(|_| FIRST_RUN.store(true, std::sync::atomic::Ordering::Relaxed))
        .on_restarted(|_| FIRST_RUN.store(true, std::sync::atomic::Ordering::Relaxed))
        .on_after_install_fast_callback(|_| report(register()))
        .on_after_update_fast_callback(|_| report(register()))
        .on_before_uninstall_fast_callback(|_| report(unregister()))
        .run();
}

fn report(result: Result<(), String>) {
    if let Err(error) = result {
        log::warn!("Windows registration: {error}");
    }
}

fn app_dir() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| e.to_string())?.parent().map(Path::to_path_buf).ok_or_else(|| "the application folder is unavailable".into())
}

fn register() -> Result<(), String> {
    let dir = app_dir()?;
    let classes = RegKey::predef(HKEY_CURRENT_USER);
    let (app, _) = classes.create_subkey(format!("Software\\Classes\\{PROGID}")).map_err(|e| e.to_string())?;
    app.set_value("FriendlyAppName", &"McMurryPDF").map_err(|e| e.to_string())?;
    let (command, _) = app.create_subkey("shell\\open\\command").map_err(|e| e.to_string())?;
    command.set_value("", &format!("\"{}\" \"%1\"", dir.join("McMurryPDF.exe").display())).map_err(|e| e.to_string())?;
    let (icon, _) = app.create_subkey("DefaultIcon").map_err(|e| e.to_string())?;
    icon.set_value("", &format!("\"{}\"", dir.join("mcmurrypdf.ico").display())).map_err(|e| e.to_string())?;
    let (types, _) = app.create_subkey("SupportedTypes").map_err(|e| e.to_string())?;
    types.set_value(".pdf", &"").map_err(|e| e.to_string())?;
    // No UserChoice edits, extra document ProgID, or duplicate OpenWithProgids entry.
    Ok(())
}

fn unregister() -> Result<(), String> {
    let root = RegKey::predef(HKEY_CURRENT_USER);
    let key = format!("Software\\Classes\\{PROGID}");
    // Remove only the registration pointing at this install; leave other copies alone.
    if let Ok(command) = root.open_subkey(format!("{key}\\shell\\open\\command")) {
        let value: String = command.get_value("").map_err(|e| e.to_string())?;
        if value == format!("\"{}\" \"%1\"", app_dir()?.join("McMurryPDF.exe").display()) {
            root.delete_subkey_all(key).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

struct Coordinator {
    manager: UpdateManager,
    available: Option<Box<UpdateInfo>>,
    downloaded: bool,
    lease: SessionLease,
}

pub fn connect(app: &mut pdfcraft_ui_egui::PdfCraftApp) {
    let Ok(state) = coordinator() else { return };
    let state = Arc::new(Mutex::new(state));
    app.installed_update_source = Some(Arc::new(move |action| state.lock().map_err(|_| "the update coordinator stopped".to_string())?.run(action)));
    if FIRST_RUN.load(std::sync::atomic::Ordering::Relaxed) && !thumbnails_current() {
        app.show_installed_updates();
    }
}

fn coordinator() -> Result<Coordinator, String> {
    let dir = super::settings_dir().ok_or("The settings folder is unavailable.")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let lease = SessionLease::installed()?.ok_or("This is not an installed McMurryPDF package.")?;
    let options = UpdateOptions::default();
    let source: Box<dyn sources::UpdateSource> = match std::env::var("MCMURRYPDF_UPDATE_SOURCE") {
        Ok(path) => Box::new(sources::FileSource::new(&path)),
        // A fixed valid URL: GithubSource's internal parser cannot reject this literal.
        Err(_) => Box::new(sources::GithubSource::new(REPOSITORY, None, true)),
    };
    let manager = UpdateManager::new_boxed(source, Some(options), None).map_err(|e| e.to_string())?;
    Ok(Coordinator { manager, available: None, downloaded: false, lease })
}

/// Headless package lifecycle diagnostics: no PDF data is accepted or transmitted.
pub fn command_line() -> Option<i32> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("--package-action") {
        return None;
    }
    let action = args.next().unwrap_or_default();
    let result = (|| -> Result<InstalledRelease, String> {
        let mut state = coordinator()?;
        if action == "state" {
            return Ok(InstalledRelease {
                current: state.manager.get_current_version_as_string(),
                available: None,
                phase: InstalledPhase::Idle,
                message: String::new(),
            });
        }
        if action == "thumbnails" {
            return state.run(InstalledAction::Thumbnails);
        }
        if !matches!(action.as_str(), "check" | "download" | "apply") {
            return Err("Use state, check, download, apply or thumbnails.".into());
        }
        let found = state.run(InstalledAction::Check)?;
        if action == "check" || found.available.is_none() {
            return Ok(found);
        }
        let staged = state.run(InstalledAction::Download)?;
        if action == "download" {
            return Ok(staged);
        }
        state.run(InstalledAction::Install)
    })();
    let (value, code) = match result {
        Ok(release) => (
            serde_json::json!({"current":release.current,"available":release.available,"phase":format!("{:?}",release.phase),"message":release.message}),
            0,
        ),
        Err(error) => (serde_json::json!({"error":error}), 1),
    };
    let _ = std::io::Write::write_fmt(&mut std::io::stdout(), format_args!("{value}\n"));
    Some(code)
}

impl Coordinator {
    fn run(&mut self, action: InstalledAction) -> Result<InstalledRelease, String> {
        let current = self.manager.get_current_version_as_string();
        let mut release = InstalledRelease {
            current,
            available: None,
            phase: InstalledPhase::Idle,
            message: if thumbnails_current() {
                String::new()
            } else {
                "Explorer thumbnails are not installed or need an update. Use the thumbnail installer below.".into()
            },
        };
        match action {
            InstalledAction::Check => {
                self.available = match self.manager.check_for_updates().map_err(|e| e.to_string())? {
                    UpdateCheck::UpdateAvailable(update) => Some(update),
                    _ => None,
                };
                self.downloaded = false;
                release.phase = InstalledPhase::Current;
            }
            InstalledAction::Download => {
                let update = self.available.as_ref().ok_or("Check for updates first.")?;
                self.manager.download_updates(update, None).map_err(|e| e.to_string())?;
                self.downloaded = true;
            }
            InstalledAction::Install => {
                let update = self.available.as_ref().filter(|_| self.downloaded).ok_or("Download the update first.")?;
                self.lease.exclusive()?;
                if let Err(error) = self.manager.wait_exit_then_apply_updates(&update.TargetFullRelease, false, true, Vec::<String>::new()) {
                    self.lease.shared()?;
                    return Err(error.to_string());
                }
                release.phase = InstalledPhase::Restarting;
                return Ok(release);
            }
            InstalledAction::Thumbnails => {
                launch_thumbnails()?;
                release.message = "Windows will request administrator approval to install or repair Explorer thumbnails.".into();
            }
        }
        if let Some(update) = &self.available {
            release.available = Some(update.TargetFullRelease.Version.clone());
            release.phase = if self.downloaded { InstalledPhase::Ready } else { InstalledPhase::Available };
        }
        Ok(release)
    }
}

fn thumbnails_current() -> bool {
    use std::io::Read;
    let Ok(dir) = app_dir() else { return false };
    let Ok(file) = File::open(dir.join("ShellVersion.txt")) else { return false };
    let mut wanted = String::new();
    if file.take(64).read_to_string(&mut wanted).is_err() || wanted.trim().is_empty() {
        return false;
    }
    let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey("Software\\McMurrySoftware\\McMurryPDF") else { return false };
    let installed: Result<String, _> = key.get_value("ShellVersion");
    installed.is_ok_and(|version| version.trim() == wanted.trim())
}

fn launch_thumbnails() -> Result<(), String> {
    let msi = app_dir()?.join("McMurryPDF-Shell.msi");
    if !msi.is_file() {
        return Err("The Explorer thumbnail installer is missing. Reinstall McMurryPDF.".into());
    }
    // Retire prototype per-user COM overrides so Windows can use the MSI's trusted machine class.
    let root = RegKey::predef(HKEY_CURRENT_USER);
    for kind in ["CLSID", "AppID"] {
        let key = format!("Software\\Classes\\{kind}\\{THUMBNAIL_CLSID}");
        if root.open_subkey(&key).is_ok() {
            root.delete_subkey_all(key).map_err(|e| e.to_string())?;
        }
    }
    let system = std::env::var_os("SystemRoot").ok_or("Windows folder is unavailable")?;
    std::process::Command::new(PathBuf::from(system).join("System32\\msiexec.exe"))
        .arg("/i")
        .arg(msi)
        .args(["/passive", "/norestart"])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
