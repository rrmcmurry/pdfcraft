use egui_kittest::{Harness, kittest::Queryable};
use pdfcraft_ui_egui::{
    PdfCraftApp,
    updates::{InstalledAction, InstalledPhase, InstalledRelease},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[path = "ui_group/updates.rs"]
mod portable_updates;

#[test]
fn unsaved_documents_never_dispatch_an_update_install() {
    let mut app = PdfCraftApp::new();
    let bytes = app.session.create_blank(612.0, 792.0, 1).unwrap();
    app.open_created_bytes("unsaved.pdf", Ok(bytes)).unwrap();
    assert!(app.first_dirty().is_some());
    let calls = Arc::new(AtomicUsize::new(0));
    let captured = calls.clone();
    app.installed_update_source = Some(Arc::new(move |_| {
        captured.fetch_add(1, Ordering::SeqCst);
        Ok(InstalledRelease { current: "0.8.0".into(), available: Some("0.8.1".into()), phase: InstalledPhase::Restarting, message: String::new() })
    }));
    app.perform_installed_update(InstalledAction::Install);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let mut harness = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_eframe(move |_| app);
    harness.run_steps(3);
    harness.get_by_label_contains("Save or close documents with unsaved changes");
}

#[test]
fn installed_updates_are_explicit_and_download_then_offer_restart() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let mut harness = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_eframe(move |_| {
        let mut app = PdfCraftApp::new();
        app.installed_update_source = Some(Arc::new(move |action| {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(InstalledRelease {
                current: "0.8.0".into(),
                available: Some("0.8.1".into()),
                phase: if action == InstalledAction::Download { InstalledPhase::Ready } else { InstalledPhase::Available },
                message: String::new(),
            })
        }));
        app
    });
    harness.run_steps(3);
    assert_eq!(calls.load(Ordering::SeqCst), 0, "startup must not contact the update feed");
    harness.state_mut().check_for_updates();
    for _ in 0..100 {
        harness.run_steps(2);
        if harness.query_by_label("Download update").is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    // Let the centered modal settle after the asynchronous result changes its size.
    harness.run_steps(3);
    harness.get_by_label("Download update").click();
    for _ in 0..100 {
        harness.run_steps(2);
        if harness.query_by_label("Install update and restart").is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    harness.get_by_label("Install update and restart");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
