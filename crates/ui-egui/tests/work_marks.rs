use egui_kittest::{Harness, kittest::Queryable};
use pdfcraft_ui_egui::{PdfCraftApp, QuickTool, RightPanel};

fn harness() -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_| {
        let mut app = PdfCraftApp::new();
        let bytes = app.session.create_blank(400.0, 500.0, 1).unwrap();
        app.open_bytes("synthetic-workpaper.pdf", None, bytes.as_ref().clone()).unwrap();
        app.set_option("language", "en").unwrap();
        app.set_option("zoom", "100").unwrap();
        app.set_option("author", "Reviewer").unwrap();
        app.choose_right_panel(Some(RightPanel::Comments));
        app.choose_right_panel(None);
        app.left_open = false;
        app.execute("comment.mark");
        app
    });
    h.run_steps(5);
    h
}

fn place(h: &mut Harness<'static, PdfCraftApp>, x: f32, y: f32) {
    let s = h.state();
    let doc = s.session.get(s.views[0].id).unwrap();
    let page = &doc.info.pages[0];
    let xf = pdfcraft_ui_egui::canvas::PageXform {
        rect: s.views[0].page_screen_rect(0).unwrap(),
        rot: s.views[0].rotation,
        pw: page.width,
        ph: page.height,
    };
    let point = xf.norm_to_screen(x / page.width, y / page.height);
    h.hover_at(point);
    h.run_steps(1);
    h.drag_at(point);
    h.run_steps(1);
    h.drop_at(point);
    h.run_steps(4);
}

#[test]
fn repeated_marks_are_comments_and_do_not_reopen_the_panel() {
    let mut h = harness();
    h.get_by_label("Mark text");
    for n in 0..15 {
        place(&mut h, 200.0, 40.0 + n as f32 * 20.0);
    }
    for (text, y) in [("B", 355.0), ("4797", 385.0), ("/", 415.0), ("D", 445.0)] {
        h.state_mut().set_option("mark-text", text).unwrap();
        h.run_steps(2);
        place(&mut h, 250.0, y);
    }
    let app = h.state();
    assert_eq!(app.quick_tool, QuickTool::Mark);
    assert_eq!(app.right, None);
    let id = app.views[0].id;
    let annotations = &app.session.get(id).unwrap().info.annotations;
    assert_eq!(annotations.len(), 19);
    assert!(annotations.iter().all(|a| a.subtype == "Stamp" && a.author.as_deref() == Some("Reviewer")));
    assert_eq!(annotations.iter().filter(|a| a.contents.as_deref() == Some("•")).count(), 15);
    for label in ["B", "4797", "✓", "D"] {
        assert!(annotations.iter().any(|a| a.contents.as_deref() == Some(label)));
    }
    let bytes = app.session.save_bytes(id).unwrap();
    let mut reopened = pdfcraft_engine::Session::new();
    let saved = reopened.open("saved.pdf", None, bytes, None).unwrap();
    assert_eq!(reopened.get(saved).unwrap().info.annotations.len(), 19);
    h.state_mut().session.undo(id).unwrap();
    assert_eq!(h.state().session.get(id).unwrap().info.annotations.len(), 18);
    h.state_mut().session.redo(id).unwrap();
    assert_eq!(h.state().session.get(id).unwrap().info.annotations.len(), 19);
    h.state_mut().execute("comment.mark");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert_eq!(h.state().quick_tool, QuickTool::Select);
}

#[test]
fn mark_entry_controls_and_rendered_preview() {
    let mut h = harness();
    assert!(!h.state().left_open, "marking must not open the tools tray");
    h.state_mut().quick_tool = QuickTool::Select;
    h.run_steps(3);
    h.get_by_label("Mark workpaper").click();
    h.run_steps(3);
    h.get_by_label("Mark text").type_text("4797");
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    assert!(!h.state().left_open);
    h.get_by_label("Mark color");
    h.get_by_label("Mark size");
    h.get_by_label("Change mark text");
    h.get_by_label("Mark size").click();
    h.run_steps(3);
    h.get_by(|n| n.role() == egui::accesskit::Role::Button && n.label().is_some_and(|s| s.starts_with("Helvetica"))).click();
    h.run_steps(3);
    if let Ok(dir) = std::env::var("MCMURRY_MARK_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        h.render().unwrap().save(std::path::Path::new(&dir).join("mark-font-picker.png")).unwrap();
    }
    h.get_by_label("Courier-Bold").click();
    h.run_steps(3);
    assert_eq!(h.state().marking.font, "Courier-Bold");
    place(&mut h, 200.0, 120.0);
    for _ in 0..30 {
        h.run_steps(2);
        if !h.state().render_pending() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    if let Ok(dir) = std::env::var("MCMURRY_MARK_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        h.render().unwrap().save(std::path::Path::new(&dir).join("marking-tool.png")).unwrap();
    }
    assert_eq!(h.state().session.get(h.state().views[0].id).unwrap().info.annotations[0].contents.as_deref(), Some("4797"));
    let settings = h.state().persist();
    let mut restored = PdfCraftApp::new();
    restored.restore(&settings);
    assert_eq!(restored.marking.text, "4797");
    assert_eq!(restored.marking.font, "Courier-Bold");
    assert_eq!(restored.quick_tool, QuickTool::Select, "remember the value, not an armed tool");
}

#[test]
fn clicking_mark_preview_reopens_and_selects_text_for_replacement() {
    let mut h = harness();
    place(&mut h, 100.0, 80.0);
    for (text, y) in [("4797", 120.0), ("B", 160.0), ("/", 200.0), (".", 240.0)] {
        h.get_by_label("Change mark text").click();
        h.run_steps(3);
        h.get_by_label("Mark text");
        // Send typing directly: the preview click must focus and select the old value.
        h.event(egui::Event::Text(text.into()));
        h.run_steps(2);
        assert_eq!(h.state().marking.text, text);
        h.key_press(egui::Key::Enter);
        h.run_steps(3);
        place(&mut h, 100.0, y);
    }
    let app = h.state();
    let annotations = &app.session.get(app.views[0].id).unwrap().info.annotations;
    let labels: Vec<_> = annotations.iter().map(|a| a.contents.as_deref().unwrap()).collect();
    assert_eq!(labels, ["•", "4797", "B", "✓", "•"]);
    assert_eq!(app.quick_tool, QuickTool::Mark);
    assert!(!app.left_open && app.right.is_none());
}

#[test]
fn mark_fonts_and_filled_tick_survive_save_and_render() {
    use pdfcraft_engine::work_marks::{FONTS, build_with_font};
    let mut appearances = std::collections::HashSet::new();
    for font in FONTS {
        let graphic = build_with_font("WTB 4797", 24.0, [0.0; 3], font).unwrap();
        let mut session = pdfcraft_engine::Session::new();
        let bytes = session.create_blank(400.0, 200.0, 1).unwrap();
        let id = session.open_new("font.pdf", bytes).unwrap();
        let page = session.get(id).unwrap().info.pages[0].clone();
        session.apply(id, graphic.place(0, &page, [200.0, 100.0], "Reviewer").unwrap()).unwrap();
        let saved = session.save_bytes(id).unwrap();
        assert!(String::from_utf8_lossy(saved.as_ref()).contains(font), "font lost on save: {font}");
        let mut renderer = pdfcraft_render::PageRenderer::new(saved, Default::default());
        let raster = renderer.render(pdfcraft_render::RenderRequest { scale: 1.0, ..Default::default() });
        assert!(raster.error.is_none());
        let mut ink = Vec::new();
        for (i, pixel) in raster.rgba.as_chunks::<4>().0.iter().enumerate() {
            if pixel[0] < 200 {
                ink.push(i);
            }
        }
        assert!(ink.len() > 100, "font did not render: {font}");
        appearances.insert(ink);
    }
    assert_eq!(appearances.len(), 12, "font choices must change the saved appearance");
    let tick = build_with_font("/", 24.0, [0.0; 3], "Helvetica").unwrap();
    let tick_serif = build_with_font("/", 24.0, [0.0; 3], "Times-Italic").unwrap();
    let tick_size = 24.0 * 0.7;
    assert!((tick.width - (tick_size + 4.0)).abs() < 0.001 && (tick.height - (tick_size + 4.0)).abs() < 0.001);
    assert_eq!(tick.source_bytes(), tick_serif.source_bytes(), "font selection must not change a tick");
    let mut renderer = pdfcraft_render::PageRenderer::new(tick.source_bytes(), Default::default());
    let raster = renderer.render(pdfcraft_render::RenderRequest { scale: 4.0, ..Default::default() });
    assert!(raster.error.is_none());
    if let Ok(dir) = std::env::var("MCMURRY_MARK_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        image::save_buffer(std::path::Path::new(&dir).join("tapered-tick.png"), &raster.rgba, raster.width, raster.height, image::ColorType::Rgba8)
            .unwrap();
    }
    let ink = |x: f64, y: f64| {
        let px = ((2.0 + x * tick_size) * 4.0) as u32;
        let py = ((tick.height - 2.0 - y * tick_size) * 4.0) as u32;
        raster.rgba[((py * raster.width + px) * 4) as usize] < 100
    };
    assert!(ink(0.30, 0.20) && ink(0.60, 0.60), "the broad bend and rising arm must be filled");
    assert!(!ink(0.20, 0.80) && !ink(0.80, 0.20), "a tick must not have crossing arms");
}

#[test]
fn generated_marks_remain_upright_and_visible_after_save_on_rotated_pages() {
    let mut bounds = Vec::new();
    for degrees in [0, 90, 180, 270] {
        let mut session = pdfcraft_engine::Session::new();
        let bytes = session.create_blank(300.0, 400.0, 1).unwrap();
        let id = session.open_new("rotation.pdf", bytes).unwrap();
        if degrees != 0 {
            session.apply(id, pdfcraft_engine::Edit::RotatePages { pages: vec![0], degrees }).unwrap();
        }
        let page = session.get(id).unwrap().info.pages[0].clone();
        for (text, at) in [("4797", [120.0, 120.0]), (".", [80.0, 180.0]), ("/", [160.0, 180.0])] {
            let graphic = pdfcraft_engine::work_marks::build(text, 16.0, [0.82, 0.12, 0.12]).unwrap();
            session.apply(id, graphic.place(0, &page, at, "Reviewer").unwrap()).unwrap();
        }
        let bytes = session.save_bytes(id).unwrap();
        let mut renderer = pdfcraft_render::PageRenderer::new(bytes, Default::default());
        let raster = renderer.render(pdfcraft_render::RenderRequest { page: 0, scale: 1.0, ..Default::default() });
        assert!(raster.error.is_none(), "{:?}", raster.error);
        let mut bbox = [u32::MAX, u32::MAX, 0, 0];
        for y in 95..145 {
            for x in 65..175 {
                let pixel = &raster.rgba[((y * raster.width + x) * 4) as usize..][..4];
                if pixel[0] > 120 && pixel[1] < 100 && pixel[2] < 100 {
                    bbox[0] = bbox[0].min(x);
                    bbox[1] = bbox[1].min(y);
                    bbox[2] = bbox[2].max(x);
                    bbox[3] = bbox[3].max(y);
                }
            }
        }
        assert!(bbox[0] < bbox[2] && bbox[2] - bbox[0] > bbox[3] - bbox[1], "missing or sideways text: {degrees}: {bbox:?}");
        bounds.push(bbox);
        for (x, y) in [(80_u32, 180_u32), (160, 180)] {
            let count = ((y - 12)..(y + 12))
                .flat_map(|yy| ((x - 12)..(x + 12)).map(move |xx| (xx, yy)))
                .filter(|(xx, yy)| {
                    let pixel = &raster.rgba[((yy * raster.width + xx) * 4) as usize..][..4];
                    pixel[0] > 120 && pixel[1] < 100
                })
                .count();
            assert!(count > 5, "missing symbol: rotation {degrees}, x {x}");
            if x == 80 {
                assert!(count < 40, "dot should have half its original diameter: {degrees}: {count} pixels");
            }
        }
    }
    for bbox in &bounds[1..] {
        for (a, b) in bbox.iter().zip(bounds[0]) {
            assert!(a.abs_diff(b) <= 2, "rotation changed placement: {bounds:?}");
        }
    }
}
