use egui_kittest::{Harness, kittest::Queryable};
use pdfcraft_ui_egui::{
    Dialog, PdfCraftApp, QuickTool,
    fill_sign::{FillTool, SavedSig},
};

fn harness() -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_| {
        let mut app = PdfCraftApp::new();
        app.session = pdfcraft_engine::Session::new().with_clock(|| 1792080000);
        let bytes = app.session.create_blank(400.0, 500.0, 1).unwrap();
        app.open_bytes("synthetic-initials.pdf", None, bytes.as_ref().clone()).unwrap();
        app.set_option("language", "en").unwrap();
        app.set_option("zoom", "100").unwrap();
        app.set_option("author", "Reviewer").unwrap();
        app.left_open = false;
        app.choose_right_panel(None);
        app.initials = Some(SavedSig::Typed("Old".into()));
        let id = app.views[0].id;
        let page = app.session.get(id).unwrap().info.pages[0].clone();
        let graphic = pdfcraft_engine::work_marks::build("RM", 36.0, [0.0; 3]).unwrap();
        app.session.apply(id, graphic.place(0, &page, [100.0, 90.0], "Fixture").unwrap()).unwrap();
        app
    });
    h.run_steps(5);
    h
}

fn point(h: &Harness<'static, PdfCraftApp>, x: f32, y: f32) -> egui::Pos2 {
    let app = h.state();
    let page = &app.session.get(app.views[0].id).unwrap().info.pages[0];
    let xf = pdfcraft_ui_egui::canvas::PageXform {
        rect: app.views[0].page_screen_rect(0).unwrap(),
        rot: app.views[0].rotation,
        pw: page.width,
        ph: page.height,
    };
    xf.norm_to_screen(x / page.width, y / page.height)
}

fn capture(h: &mut Harness<'static, PdfCraftApp>) {
    h.state_mut().execute("sign.off.capture");
    h.run_steps(3);
    finish_crop(h);
}

fn finish_crop(h: &mut Harness<'static, PdfCraftApp>) {
    let (a, b) = (point(h, 40.0, 50.0), point(h, 160.0, 130.0));
    h.hover_at(a);
    h.run_steps(1);
    h.drag_at(a);
    h.run_steps(2);
    h.hover_at(b);
    h.run_steps(2);
    h.drop_at(b);
    h.run_steps(5);
    assert_eq!(h.state().dialog, Some(Dialog::Signature));
}

fn place(h: &mut Harness<'static, PdfCraftApp>, x: f32, y: f32) {
    let p = point(h, x, y);
    h.hover_at(p);
    h.run_steps(1);
    h.drag_at(p);
    h.run_steps(1);
    h.drop_at(p);
    h.run_steps(4);
}

#[test]
fn crop_confirm_signoff_and_replace_preserve_old_dates_and_initials() {
    let mut h = harness();
    capture(&mut h);
    assert_eq!(h.state().initials, Some(SavedSig::Typed("Old".into())));
    h.get_by_label("Cancel").click();
    h.run_steps(3);
    assert_eq!(h.state().initials, Some(SavedSig::Typed("Old".into())));
    capture(&mut h);
    if let Ok(dir) = std::env::var("MCMURRY_SIGNOFF_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        h.render().unwrap().save(std::path::Path::new(&dir).join("capture-preview.png")).unwrap();
    }
    h.get_by_label("Apply").click();
    h.run_steps(4);
    let Some(SavedSig::Image(image)) = &h.state().initials else { panic!("captured initials were not accepted") };
    assert!(image.rgba().as_chunks::<4>().0.iter().any(|p| p[3] == 0));
    assert!(image.rgba().as_chunks::<4>().0.iter().any(|p| p[3] > 200));
    let mut source_renderer =
        pdfcraft_render::PageRenderer::new(h.state().session.get(h.state().views[0].id).unwrap().bytes.clone(), Default::default());
    let source = source_renderer.render(pdfcraft_render::RenderRequest {
        scale: 4.0,
        tile: Some(pdfcraft_render::Tile { x: 160, y: 200, w: 480, h: 320 }),
        ..Default::default()
    });
    assert!(source.error.is_none());
    let original_ink = source.rgba.as_chunks::<4>().0.iter().filter(|p| p[0] < 100).count();
    let captured_ink = image.rgba().as_chunks::<4>().0.iter().filter(|p| p[0] < 100 && p[3] > 250).count();
    assert!(captured_ink * 100 >= original_ink * 95, "capture must preserve solid strokes: {captured_ink}/{original_ink}");
    assert_eq!(h.state().quick_tool, QuickTool::Fill(FillTool::InitialsDate));
    place(&mut h, 200.0, 220.0);
    let id = h.state().views[0].id;
    let first = h.state().session.save_bytes(id).unwrap();
    let settings = h.state().persist();
    let mut restored = PdfCraftApp::new();
    restored.restore(&settings);
    assert_eq!(restored.initials, h.state().initials);
    h.state_mut().session = std::mem::replace(&mut h.state_mut().session, pdfcraft_engine::Session::new()).with_clock(|| 1792166400);
    h.state_mut().initials = Some(SavedSig::Typed("New".into()));
    place(&mut h, 200.0, 280.0);
    assert_eq!(h.state().quick_tool, QuickTool::Fill(FillTool::InitialsDate));
    assert!(h.state().right.is_none() && !h.state().left_open);
    let labels: Vec<_> = h
        .state()
        .session
        .get(id)
        .unwrap()
        .info
        .annotations
        .iter()
        .filter_map(|a| a.contents.as_deref())
        .filter(|s| s.starts_with("Initials"))
        .collect();
    assert_eq!(labels, ["Initials — 10/15/26", "Initials — 10/16/26"]);
    let saved = h.state().session.save_bytes(id).unwrap();
    let mut reopened = pdfcraft_engine::Session::new();
    let reopened_id = reopened.open("saved.pdf", None, saved.clone(), None).unwrap();
    assert_eq!(reopened.get(reopened_id).unwrap().info.annotations.len(), 3);
    h.state_mut().session.undo(id).unwrap();
    assert_eq!(h.state().session.get(id).unwrap().info.annotations.len(), 2);
    h.state_mut().session.redo(id).unwrap();
    assert_eq!(h.state().session.get(id).unwrap().info.annotations.len(), 3);
    let render = |bytes| {
        let mut r = pdfcraft_render::PageRenderer::new(bytes, Default::default());
        let out = r.render(pdfcraft_render::RenderRequest { scale: 1.0, ..Default::default() });
        assert!(out.error.is_none());
        out
    };
    let (before, after) = (render(first), render(saved));
    // The first placement's pixels stay identical after the date and saved initials change.
    for y in 200..240 {
        let range = (y * 400 * 4)..((y + 1) * 400 * 4);
        assert_eq!(&before.rgba[range.clone()], &after.rgba[range]);
    }
    if let Ok(dir) = std::env::var("MCMURRY_SIGNOFF_SHOTS") {
        h.render().unwrap().save(std::path::Path::new(&dir).join("signoff-placed.png")).unwrap();
    }
}

#[test]
fn capture_from_drawn_initials_dismisses_the_edit_dialog() {
    let mut h = harness();
    let saved = SavedSig::Drawn(vec![vec![[0.1, 0.1], [0.2, 0.2], [0.3, 0.1]]]);
    h.state_mut().initials = Some(saved.clone());
    h.state_mut().execute("sign.off");
    h.run_steps(3);
    assert!(h.query_by_label("Capture initials from page").is_none(), "no standalone crop shortcut");
    h.state_mut().execute("sign.off.change");
    h.run_steps(3);
    assert!(h.query_by_label("Scan").is_none(), "initials use Capture in the method row");
    if let Ok(dir) = std::env::var("MCMURRY_SIGNOFF_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        h.render().unwrap().save(std::path::Path::new(&dir).join("initials-capture-tab.png")).unwrap();
    }
    h.get_by_label("Capture").click();
    h.run_steps(3);
    assert_eq!(h.state().dialog, None, "Capture must dismiss the modal before starting the crop gesture");
    assert_eq!(h.state().quick_tool, QuickTool::CaptureInitials);
    assert_eq!(h.state().initials, Some(saved));
    finish_crop(&mut h);
    h.get_by_label("Apply").click();
    h.run_steps(3);
    assert!(matches!(h.state().initials, Some(SavedSig::Image(_))));
    assert_eq!(h.state().quick_tool, QuickTool::Fill(FillTool::InitialsDate));
}

#[test]
fn empty_invalid_and_outside_crops_are_refused_without_replacing_initials() {
    let h = harness();
    let app = h.state();
    let id = app.views[0].id;
    for rect in [[200.0, 300.0, 250.0, 350.0], [-1.0, 0.0, 50.0, 50.0], [0.0, 0.0, f64::NAN, 50.0], [0.0, 0.0, 1.0, 1.0]] {
        assert!(app.session.capture_initials(id, 0, rect).is_err());
    }
    assert_eq!(app.initials, Some(SavedSig::Typed("Old".into())));
}
