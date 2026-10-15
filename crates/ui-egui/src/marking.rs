//! Persistent click-to-place workpaper marks, without a stamp library.
use egui::Color32;
use pdfcraft_engine::work_marks::Graphic;
use serde::{Deserialize, Serialize};

type CachedGraphic = (String, String, f64, [f64; 3], Result<Graphic, String>);

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Marking {
    pub text: String,
    pub size: f64,
    pub color: [f64; 3],
    pub font: String,
    #[serde(skip)]
    pub(crate) focus: bool,
    #[serde(skip)]
    cached: Option<CachedGraphic>,
    #[serde(skip)]
    preview: Option<egui::TextureHandle>,
}

impl Default for Marking {
    fn default() -> Self {
        Self {
            text: ".".into(),
            size: 12.0,
            color: [0.82, 0.12, 0.12],
            font: pdfcraft_engine::work_marks::DEFAULT_FONT.into(),
            focus: false,
            cached: None,
            preview: None,
        }
    }
}

impl Marking {
    pub fn graphic(&mut self) -> Result<Graphic, String> {
        if let Some((text, font, size, color, result)) = &self.cached
            && *text == self.text
            && *font == self.font
            && *size == self.size
            && *color == self.color
        {
            return result.clone();
        }
        let result = pdfcraft_engine::work_marks::build_with_font(&self.text, self.size, self.color, &self.font);
        self.preview = None;
        self.cached = Some((self.text.clone(), self.font.clone(), self.size, self.color, result.clone()));
        result
    }

    pub(crate) fn preview(&mut self, ctx: &egui::Context) -> Option<egui::TextureHandle> {
        let graphic = self.graphic().ok()?;
        if self.preview.is_none() {
            let mut renderer = pdfcraft_render::PageRenderer::new(graphic.source_bytes(), Default::default());
            let raster = renderer.render(pdfcraft_render::RenderRequest { scale: 2.0, ..Default::default() });
            if raster.error.is_some() || raster.rgba.is_empty() {
                return None;
            }
            let rgb = self.color.map(|v| (v * 255.0).round() as u8);
            let ink = 255.0 - f64::from(rgb.into_iter().min().unwrap_or(0));
            let pixels = raster
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|pixel| {
                    // Each array has four channels, so all RGB indices exist.
                    let coverage = if ink > 0.0 { (255.0 - f64::from(pixel[0].min(pixel[1]).min(pixel[2]))) / ink } else { 0.0 };
                    Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], (coverage.clamp(0.0, 1.0) * 255.0).round() as u8)
                })
                .collect();
            let image = egui::ColorImage::new([raster.width as usize, raster.height as usize], pixels);
            self.preview = Some(ctx.load_texture("work-mark-preview", image, egui::TextureOptions::LINEAR));
        }
        self.preview.clone()
    }
}

/// Use the same narrow toolbar extension and anchored property menus as comment tools.
pub(crate) fn quick_bar_controls(app: &mut crate::PdfCraftApp, ui: &mut egui::Ui) {
    let mut focus = std::mem::take(&mut app.marking.focus);
    let label = app.marking.graphic().map_or_else(|_| "!".into(), |g| g.label);
    let text = ui.add_sized([32.0, 32.0], egui::Button::new(egui::RichText::new(label).size(11.0)).frame(false).truncate());
    text.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Change mark text"));
    focus |= text.clicked();
    let text = crate::theme::hand(text).on_hover_text(format!("Mark: {}\nClick to edit (. = dot; / = checkmark)", app.marking.text));
    egui::Popup::menu(&text)
        .open_memory(focus.then_some(egui::SetOpenCommand::Bool(true)))
        .align(egui::RectAlign::RIGHT_START)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            crate::widgets::begin_dialog(ui);
            text_entry(app, ui, focus);
        });

    let color = crate::comments::color32(app.marking.color);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
    ui.painter().circle_filled(rect.center(), 9.0, color);
    ui.painter().circle_stroke(rect.center(), 9.0, egui::Stroke::new(1.0, Color32::from_black_alpha(50)));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Mark color"));
    let response = response.on_hover_text("Mark color");
    egui::Popup::menu(&response).align(egui::RectAlign::RIGHT_START).show(|ui| {
        if let Some(color) = crate::comments::swatch_grid(ui, Some(app.marking.color)) {
            app.marking.color = color;
            ui.close();
        }
    });
    let size = crate::icons::button(ui, "type", 32.0, false, "Mark size");
    egui::Popup::menu(&size).align(egui::RectAlign::RIGHT_START).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
        ui.set_min_width(180.0);
        ui.horizontal(|ui| {
            ui.label("Font");
            let selected = app.marking.font.clone();
            // A submenu stays in the property popup's menu tree; a ComboBox opens a
            // competing memory popup and dismisses the parent before selection.
            ui.menu_button(selected, |ui| {
                for font in pdfcraft_engine::work_marks::FONTS {
                    if ui.selectable_value(&mut app.marking.font, font.into(), font).clicked() {
                        ui.close();
                    }
                }
            });
        });
        ui.add(egui::Slider::new(&mut app.marking.size, 6.0..=36.0).suffix(" pt"));
    });
}

fn text_entry(app: &mut crate::PdfCraftApp, ui: &mut egui::Ui, focus: bool) {
    let label = ui.label("Mark text");
    let mut output = crate::widgets::line(&mut app.marking.text).char_limit(pdfcraft_engine::work_marks::MAX_CHARS).desired_width(128.0).show(ui);
    let response = egui::Response::clone(&output.response).labelled_by(label.id);
    response.widget_info(|| {
        let mut info = egui::WidgetInfo::new(egui::WidgetType::TextEdit);
        info.label = Some("Mark text".into());
        info.current_text_value = Some(app.marking.text.clone());
        info
    });
    response.clone().on_hover_text("Type a reference, then Enter and click to mark. A lone . makes a dot; / makes a checkmark. Esc stops marking.");
    if focus {
        response.request_focus();
    }
    if focus || response.gained_focus() {
        output.state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(0),
            egui::text::CCursor::new(app.marking.text.chars().count()),
        )));
        output.state.store(ui.ctx(), response.id);
    }
    if response.changed() {
        app.quick_tool = crate::QuickTool::Mark;
    }
    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        app.quick_tool = crate::QuickTool::Mark;
        ui.close();
    }
    if let Err(e) = app.marking.graphic() {
        ui.colored_label(Color32::DARK_RED, e);
    }
}
