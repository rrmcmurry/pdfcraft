//! Help ▸ Check for updates: explicit release checks and installed-package updates.
//!
//! The desktop app supplies how to ask ([`PdfCraftApp::update_source`]), so this crate has no
//! network code; without a source (the web build, tests) the command opens the releases page.
//! Installed Windows hosts also supply download/install operations; portable hosts offer a release page.
//! It asks only when the user does: there is no check at start (the owner's decision).

use std::sync::Arc;

use egui::{Align, Layout};

use crate::{PdfCraftApp, theme, widgets};

/// Where every PdfCraft release is listed.
pub const RELEASES_PAGE: &str = "https://github.com/rrmcmurry/pdfcraft/releases";

/// The latest published release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// Its version tag, such as `v0.2.0`.
    pub version: String,
    /// Its page on [`RELEASES_PAGE`], where the downloads are.
    pub url: String,
}

/// Asks for the latest release (blocking; it runs on its own thread).
pub type UpdateSource = Arc<dyn Fn() -> Result<Release, String> + Send + Sync>;

/// Explicit user operations supported by installed Windows packages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstalledAction {
    Check,
    Download,
    Install,
    Thumbnails,
}

/// Installed package state; the native host owns download and installation details.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstalledPhase {
    Idle,
    Current,
    Available,
    Ready,
    Restarting,
}

#[derive(Clone, Debug)]
pub struct InstalledRelease {
    pub current: String,
    pub available: Option<String>,
    pub phase: InstalledPhase,
    pub message: String,
}

pub type InstalledUpdateSource = Arc<dyn Fn(InstalledAction) -> Result<InstalledRelease, String> + Send + Sync>;

/// Whether release `latest` (a tag such as `v0.2.0`) is newer than version `current` (`0.1.1`).
/// Pre-release and build suffixes are ignored; a version that doesn't parse is never newer.
pub fn is_newer(latest: &str, current: &str) -> bool {
    matches!((parse(latest), parse(current)), (Some(l), Some(c)) if l > c)
}

fn parse(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let core = v.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let mut next = |required: bool| match parts.next() {
        Some(p) => p.parse::<u64>().ok(),
        None if required => None,
        None => Some(0),
    };
    let version = (next(true)?, next(false)?, next(false)?);
    parts.next().is_none().then_some(version)
}

/// Where a check is.
#[derive(Default)]
pub(crate) enum Check {
    #[default]
    Idle,
    #[cfg(not(target_arch = "wasm32"))]
    Running(std::sync::mpsc::Receiver<Result<Release, String>>),
    Done(Result<Release, String>),
}

#[derive(Default)]
pub(crate) struct Updates {
    pub(crate) check: Check,
    /// The Updates dialog is showing.
    pub(crate) open: bool,
    installed: Option<Result<InstalledRelease, String>>,
    #[cfg(not(target_arch = "wasm32"))]
    installed_task: Option<std::sync::mpsc::Receiver<Result<InstalledRelease, String>>>,
    busy: Option<&'static str>,
    error: Option<String>,
}

impl PdfCraftApp {
    /// Help ▸ Check for updates: ask for the latest release and show the outcome.
    pub fn check_for_updates(&mut self) {
        if self.installed_update_source.is_some() {
            self.perform_installed_update(InstalledAction::Check);
            return;
        }
        let Some(source) = self.update_source.clone() else {
            self.open_url(RELEASES_PAGE);
            return;
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.updates.open = true;
            if matches!(self.updates.check, Check::Running(_)) {
                return;
            }
            let (tx, rx) = std::sync::mpsc::channel();
            let ctx = self.ctx.clone();
            std::thread::spawn(move || {
                // The receiver may be gone (the app quit): nothing to report to then.
                let _ = tx.send(source());
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            });
            self.updates.check = Check::Running(rx);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = source;
            self.open_url(RELEASES_PAGE);
        }
    }

    /// Pick up a finished check (each frame).
    pub(crate) fn poll_updates(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(rx) = &self.updates.installed_task {
            match rx.try_recv() {
                Ok(result) => {
                    self.updates.installed = Some(result);
                    self.updates.busy = None;
                    self.updates.installed_task = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.updates.installed = Some(Err("The update operation stopped unexpectedly.".into()));
                    self.updates.busy = None;
                    self.updates.installed_task = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Check::Running(rx) = &self.updates.check {
            let result = match rx.try_recv() {
                Ok(r) => r,
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => Err("the update check stopped unexpectedly".into()),
            };
            self.updates.check = Check::Done(result);
        }
    }

    /// Open package controls without contacting an update server.
    pub fn show_installed_updates(&mut self) {
        self.updates.open = true;
    }

    /// Downloads run in the background. Installation is dispatched only with clean documents.
    pub fn perform_installed_update(&mut self, action: InstalledAction) {
        let Some(source) = self.installed_update_source.clone() else { return };
        self.updates.open = true;
        if self.updates.busy.is_some() {
            return;
        }
        self.updates.error = None;
        if action == InstalledAction::Install {
            if self.first_dirty().is_some() {
                self.updates.error = Some("Save or close documents with unsaved changes before installing an update.".into());
                return;
            }
            match source(action) {
                Ok(result) if result.phase == InstalledPhase::Restarting => {
                    self.updates.installed = Some(Ok(result));
                    if let Some(ctx) = &self.ctx {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                Ok(result) => self.updates.installed = Some(Ok(result)),
                Err(error) => self.updates.error = Some(error),
            }
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            let ctx = self.ctx.clone();
            self.updates.busy = Some(match action {
                InstalledAction::Download => "Downloading and verifying the update…",
                InstalledAction::Thumbnails => "Opening the Explorer thumbnail installer…",
                _ => "Checking for a newer version…",
            });
            self.updates.installed_task = Some(rx);
            std::thread::spawn(move || {
                let _ = tx.send(source(action));
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            });
        }
    }
}

/// The Updates dialog.
pub(crate) fn dialog(app: &mut PdfCraftApp, ctx: &egui::Context) {
    if !app.updates.open {
        return;
    }
    if app.installed_update_source.is_some() {
        installed_dialog(app, ctx);
        return;
    }
    let t = theme::Tokens::get(ctx);
    let current = env!("CARGO_PKG_VERSION");
    let mut close = false;
    let mut download: Option<String> = None;
    let modal = crate::widgets::show_modal(ctx, "updates", |ui| {
        ui.set_width(420.0);
        ui.horizontal(|ui| {
            ui.add(crate::icons::image("cloud", 22.0, t.accent));
            ui.label(egui::RichText::new(tl!("Check for updates")).font(theme::semibold(16.0)));
        });
        ui.add_space(8.0);
        match &app.updates.check {
            Check::Idle => {
                ui.label(tl!("No check has run yet."));
            }
            #[cfg(not(target_arch = "wasm32"))]
            Check::Running(_) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tl!("Checking for a newer version…"));
                });
            }
            Check::Done(Ok(r)) if is_newer(&r.version, current) => {
                let version = r.version.trim_start_matches(['v', 'V']);
                ui.label(egui::RichText::new(crate::i18n::fmt(tl!("McMurryPDF {v} is available."), &[("v", version)])).strong());
                ui.label(
                    egui::RichText::new(crate::i18n::fmt(
                        tl!("You have version {c}. Download the new version from its release page."),
                        &[("c", current)],
                    ))
                    .color(t.text_muted),
                );
                download = Some(r.url.clone());
            }
            Check::Done(Ok(_)) => {
                ui.label(crate::i18n::fmt(tl!("McMurryPDF {c} is up to date."), &[("c", current)]));
            }
            Check::Done(Err(e)) => {
                ui.label(crate::i18n::fmt(tl!("Couldn't check for updates: {e}"), &[("e", &e.to_string())]));
                ui.label(
                    egui::RichText::new(crate::i18n::fmt(
                        tl!("You have version {c}. All releases are listed at {page}."),
                        &[("c", current), ("page", RELEASES_PAGE)],
                    ))
                    .color(t.text_muted),
                );
            }
        }
        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(tl!("Asks GitHub for the latest release. Nothing is downloaded or installed automatically."))
                .color(t.text_muted)
                .small(),
        );
        ui.add_space(12.0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if let Some(url) = download.take() {
                let get = widgets::pill_button(ui, tl!("Download"), true).clicked();
                let later = widgets::pill_button(ui, tl!("Later"), false).clicked();
                close = get || later;
                download = get.then_some(url);
            } else if widgets::pill_button(ui, tl!("Close"), true).clicked() {
                close = true;
            }
        });
    });
    if modal.should_close() {
        close = true;
        download = None;
    }
    if close {
        app.updates.open = false;
        if let Some(url) = download {
            app.open_url(&url);
        }
    }
}

fn installed_dialog(app: &mut PdfCraftApp, ctx: &egui::Context) {
    let mut close = false;
    let mut action = None;
    let modal = widgets::show_modal(ctx, "installed-updates", |ui| {
        ui.set_width(460.0);
        ui.heading("McMurryPDF updates");
        ui.add_space(8.0);
        if let Some(busy) = app.updates.busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(busy);
            });
        } else {
            match &app.updates.installed {
                Some(Ok(release)) => {
                    ui.label(format!("Installed version: {}", release.current));
                    if let Some(version) = &release.available {
                        ui.label(format!("McMurryPDF {version} is available."));
                    }
                    if !release.message.is_empty() {
                        ui.label(&release.message);
                    }
                    match release.phase {
                        InstalledPhase::Idle => {
                            ui.label("Check for updates when you are ready.");
                        }
                        InstalledPhase::Current => {
                            ui.label("You are up to date.");
                        }
                        InstalledPhase::Available => {
                            if widgets::pill_button(ui, "Download update", true).clicked() {
                                action = Some(InstalledAction::Download);
                            }
                        }
                        InstalledPhase::Ready => {
                            ui.label("The update is downloaded and verified. Save your work and close other McMurryPDF windows before restarting.");
                            if widgets::pill_button(ui, "Install update and restart", true).clicked() {
                                action = Some(InstalledAction::Install);
                            }
                        }
                        InstalledPhase::Restarting => {
                            ui.label("Restarting to install the update…");
                        }
                    }
                }
                Some(Err(error)) => {
                    ui.label(format!("Could not complete the update: {error}"));
                }
                None => {
                    ui.label("Check for updates when you are ready.");
                }
            }
            if let Some(error) = &app.updates.error {
                ui.label(error);
            }
            ui.add_space(8.0);
            if widgets::pill_button(ui, "Check for updates", false).clicked() {
                action = Some(InstalledAction::Check);
            }
            ui.separator();
            ui.label("Explorer thumbnails work without Acrobat. Installing or repairing this Windows component requires administrator approval.");
            if widgets::pill_button(ui, "Install or repair Explorer thumbnails", false).clicked() {
                action = Some(InstalledAction::Thumbnails);
            }
        }
        ui.add_space(8.0);
        ui.small("Update checks send release information only. Your PDFs stay on this computer. Updates are installed only when you choose.");
        if widgets::pill_button(ui, "Close", false).clicked() {
            close = true;
        }
    });
    if close || modal.should_close() {
        app.updates.open = false;
    }
    if let Some(action) = action {
        app.perform_installed_update(action);
    }
}
