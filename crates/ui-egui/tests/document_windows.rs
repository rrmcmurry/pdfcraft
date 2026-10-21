use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::PdfCraftApp;
use std::sync::{Arc, Mutex};

fn one_document() -> PdfCraftApp {
    let mut app = PdfCraftApp::new();
    let bytes = app.session.create_blank(612.0, 792.0, 1).unwrap();
    app.open_bytes("first.pdf", None, bytes.as_ref().clone()).unwrap();
    app
}

#[test]
fn preference_round_trips_and_old_settings_keep_tabs() {
    let mut app = PdfCraftApp::new();
    app.set_option("open-tabs", "off").unwrap();
    let mut restored = PdfCraftApp::new();
    restored.restore(&app.persist());
    assert!(!restored.open_documents_in_tabs);
    let mut legacy = PdfCraftApp::new();
    legacy.restore("{}");
    assert!(legacy.open_documents_in_tabs);
    assert!(app.set_option("open-tabs", "maybe").is_err());
    assert!(!app.open_documents_in_tabs);
}

#[test]
fn separate_window_dispatch_keeps_the_existing_document() {
    let mut app = one_document();
    let original = app.views[0].id;
    let requests = Arc::new(Mutex::new(Vec::new()));
    let capture = requests.clone();
    app.document_window_source = Some(Box::new(move |path, tool| {
        capture.lock().unwrap().push((path.to_owned(), tool.map(str::to_owned)));
        Ok(())
    }));
    app.set_option("open-tabs", "off").unwrap();
    app.open_path("second.pdf");
    app.open_path("third.pdf");
    assert_eq!(app.views.len(), 1);
    assert_eq!(app.views[0].id, original);
    assert_eq!(requests.lock().unwrap().len(), 2);
    assert_eq!(app.last_window_open.as_deref(), Some("third.pdf"));
}

#[test]
fn failed_window_launch_preserves_the_open_document_and_reports_failure() {
    let mut app = one_document();
    let original = app.views[0].id;
    app.document_window_source = Some(Box::new(|_, _| Err("launcher unavailable".into())));
    app.set_option("open-tabs", "off").unwrap();
    app.open_path("second.pdf");
    assert_eq!(app.views[0].id, original);
    assert!(app.last_window_open.is_none());
    assert!(app.toast.as_ref().unwrap().0.contains("launcher unavailable"));
}

#[test]
fn generated_document_is_transferred_without_replacing_the_current_document() {
    let mut app = one_document();
    let original = app.views[0].id;
    let output = Arc::new(Mutex::new(Vec::new()));
    let capture = output.clone();
    app.created_document_window_source = Some(Box::new(move |name, bytes| {
        capture.lock().unwrap().push((name.to_owned(), bytes.to_vec()));
        Ok(())
    }));
    app.set_option("open-tabs", "off").unwrap();
    let bytes = app.session.create_blank(200.0, 300.0, 1).unwrap();
    app.open_created_bytes("Untitled.pdf", Ok(bytes)).unwrap();
    assert_eq!(app.views.len(), 1);
    assert_eq!(app.views[0].id, original);
    let output = output.lock().unwrap();
    assert_eq!(output[0].0, "Untitled.pdf");
    assert!(output[0].1.starts_with(b"%PDF-"));
}

#[test]
fn recovering_a_document_keeps_its_original_save_path_and_the_current_document() {
    use pdfcraft_ui_egui::{RecoveryMeta, RecoveryStore};
    let dir = std::env::temp_dir().join(format!(
        "mcmurry-recovery-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let store = RecoveryStore::new(&dir);
    let mut app = one_document();
    let original = app.views[0].id;
    let bytes = app.session.create_blank(200.0, 300.0, 1).unwrap();
    store
        .write(
            &RecoveryMeta {
                key: "snapshot".into(),
                name: "recovered.pdf".into(),
                path: Some("C:/fictional/recovered.pdf".into()),
                saved_at: 1,
                encrypted: false,
            },
            &bytes,
        )
        .unwrap();
    app.enable_recovery(store.clone());
    app.created_document_window_source = Some(Box::new(|_, _| Ok(())));
    app.set_option("open-tabs", "off").unwrap();
    app.recover(&["snapshot".into()]);
    assert_eq!(app.views.len(), 2);
    assert_eq!(app.session.get(original).unwrap().path, None);
    let recovered = app.session.get(app.views[1].id).unwrap();
    assert_eq!(recovered.path.as_deref(), Some("C:/fictional/recovered.pdf"));
    assert!(recovered.dirty);
    store.clear();
    std::fs::remove_dir(&dir).unwrap();
}

#[test]
fn checkbox_is_available_and_document_tab_is_hidden_when_off() {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 1000.0)).build_eframe(|_cc| {
        let mut app = one_document();
        app.document_window_source = Some(Box::new(|_, _| Ok(())));
        app.set_option("language", "en").unwrap();
        app.set_option("open-tabs", "off").unwrap();
        app.set_option("dialog", "preferences").unwrap();
        app
    });
    h.run_steps(3);
    assert!(h.query_by_label("Open documents as tabs in the same window").is_some());
    assert!(h.query_by_label("first.pdf").is_none());
}

#[test]
fn os_open_focuses_only_the_window_that_receives_the_file() {
    use pdfcraft_ui_egui::OsEvent;
    for mode in ["tabs", "window", "failure", "empty"] {
        let mut app = one_document();
        let path = std::env::temp_dir().join(format!("mcmurry-focus-{}-{mode}.pdf", std::process::id()));
        let bytes = app.session.create_blank(300.0, 400.0, 1).unwrap();
        std::fs::write(&path, bytes.as_slice()).unwrap();
        app.set_option("open-tabs", if mode == "tabs" { "on" } else { "off" }).unwrap();
        let failed = mode == "failure";
        app.document_window_source = Some(Box::new(move |_, _| if failed { Err("test launch failure".into()) } else { Ok(()) }));
        let mut h = Harness::builder().with_size(egui::vec2(1000.0, 700.0)).build_eframe(|_| app);
        h.run_steps(3);
        let mut paths = Some(if mode == "empty" { Vec::new() } else { vec![path.to_string_lossy().into_owned()] });
        h.state_mut().os_events = Some(Box::new(move || paths.take().map(|paths| vec![OsEvent::Open(paths)]).unwrap_or_default()));
        h.step();
        let focus = h.output().viewport_output.values().any(|v| v.commands.iter().any(|c| matches!(c, egui::ViewportCommand::Focus)));
        let restored = h.output().viewport_output.values().any(|v| v.commands.iter().any(|c| matches!(c, egui::ViewportCommand::Minimized(false))));
        assert_eq!((focus, restored), (mode == "tabs" || mode == "failure", mode == "tabs" || mode == "failure"), "{mode}");
        if mode == "window" {
            assert_eq!(h.state().views.len(), 1);
            assert_eq!(h.state().last_window_open.as_deref(), path.to_str());
        }
        std::fs::remove_file(path).unwrap();
    }
}
