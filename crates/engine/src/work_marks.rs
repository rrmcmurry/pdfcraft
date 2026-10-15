//! Short workpaper references generated locally as portable PDF stamp appearances.
use std::sync::Arc;

use crate::{Edit, FillMark, MarkFile, NewAnnotation, Rgb, Shape, Style};
use pdfcraft_render::PageInfo;

pub const MAX_CHARS: usize = 32;
pub const DEFAULT_FONT: &str = "Helvetica";
pub const FONTS: [&str; 12] = [
    "Helvetica",
    "Helvetica-Bold",
    "Helvetica-Oblique",
    "Helvetica-BoldOblique",
    "Times-Roman",
    "Times-Bold",
    "Times-Italic",
    "Times-BoldItalic",
    "Courier",
    "Courier-Bold",
    "Courier-Oblique",
    "Courier-BoldOblique",
];
/// A filled tick silhouette; shared by the PDF appearance and pointer preview.
pub const CHECK_POINTS: [[f64; 2]; 10] =
    [[0.02, 0.42], [0.26, 0.28], [0.42, 0.51], [0.62, 0.71], [0.81, 0.86], [0.98, 0.98], [0.79, 0.77], [0.62, 0.53], [0.46, 0.28], [0.29, 0.02]];

pub fn font(name: &str) -> Result<pdfcraft_fonts::Std14, String> {
    if FONTS.contains(&name)
        && let Some(font) = pdfcraft_fonts::Std14::from_base_font(name)
    {
        return Ok(font);
    }
    Err(format!("Choose a supported mark font: {}.", FONTS.join(", ")))
}

#[derive(Clone, Debug)]
pub struct Graphic {
    pub label: String,
    pub width: f64,
    pub height: f64,
    file: MarkFile,
}

impl Graphic {
    pub(crate) fn from_source(label: String, width: f64, height: f64, bytes: Vec<u8>) -> Self {
        Self { label, width, height, file: MarkFile { name: "signoff.pdf".into(), bytes: Arc::new(bytes), page: 0 } }
    }
    pub fn source_bytes(&self) -> Arc<Vec<u8>> {
        self.file.bytes.clone()
    }

    /// Centre on displayed-page coordinates, including crop, rotation and UserUnit.
    pub fn place(&self, page: usize, info: &PageInfo, at: [f64; 2], author: &str) -> Result<Edit, String> {
        if at.iter().any(|v| !v.is_finite()) || at[0] < 0.0 || at[1] < 0.0 || at[0] > f64::from(info.width) || at[1] > f64::from(info.height) {
            return Err("Place the mark inside the page.".into());
        }
        let a = info.view_to_user((at[0] - self.width / 2.0) as f32, (at[1] - self.height / 2.0) as f32);
        let b = info.view_to_user((at[0] + self.width / 2.0) as f32, (at[1] + self.height / 2.0) as f32);
        let rect = [f64::from(a[0].min(b[0])), f64::from(a[1].min(b[1])), f64::from(a[0].max(b[0])), f64::from(a[1].max(b[1]))];
        Ok(Edit::AddCustomStamp { page, rect, name: self.label.clone(), file: self.file.clone(), author: author.into() })
    }
}

/// Build once when text/style changes; repeated placements share the tiny source PDF.
pub fn build(text: &str, size: f64, color: Rgb) -> Result<Graphic, String> {
    build_with_font(text, size, color, DEFAULT_FONT)
}

pub fn build_with_font(text: &str, size: f64, color: Rgb, font_name: &str) -> Result<Graphic, String> {
    let font = font(font_name)?;
    let text = text.trim();
    if text.is_empty() || text.chars().count() > MAX_CHARS || text.chars().any(char::is_control) {
        return Err(format!("Enter a single-line mark of 1–{MAX_CHARS} characters."));
    }
    if !size.is_finite() || !(6.0..=36.0).contains(&size) || color.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)) {
        return Err("Choose a size from 6 to 36 points and a valid color.".into());
    }
    let symbol = match text {
        "." => Some((FillMark::Dot, "•")),
        "/" => Some((FillMark::Check, "✓")),
        _ => None,
    };
    if symbol.is_none()
        && let Some(c) = pdfcraft_fonts::first_non_win_ansi(text)
    {
        return Err(format!("The mark font cannot draw {c:?}. Use / for a checkmark or . for a dot."));
    }
    // A dot is a small workpaper tick, not a full-height text character.
    let symbol_size = match text {
        "." => size * 0.5,
        "/" => size * 0.7,
        _ => size,
    };
    let (width, height) =
        if symbol.is_some() { (symbol_size + 4.0, symbol_size + 4.0) } else { (font.text_width(text, size) + size * 0.7 + 8.0, size * 1.4 + 8.0) };
    let shape = match symbol {
        Some((mark, _)) => Shape::Mark { rect: [2.0, 2.0, width - 2.0, height - 2.0], mark },
        None => Shape::TextBox { rect: [0.0, 0.0, width, height], font_size: size },
    };
    let label = symbol.map_or(text, |(_, label)| label).to_string();
    let style = Style { color, width: if symbol.is_some() { 1.4 } else { 0.0 }, fill: None, opacity: 1.0 };
    let mut doc = pdfcraft_create::blank(width, height, 1).map_err(|e| e.to_string())?;
    if text == "." {
        pdfcraft_annot::add_annotation(
            &mut doc,
            &NewAnnotation { page: 0, shape, style, contents: text.into(), author: String::new() },
            &pdfcraft_annot::Meta::default(),
        )
        .map_err(|e| e.to_string())?;
        pdfcraft_edit::flatten(&mut doc, &[0], true, false).map_err(|e| e.to_string())?;
    } else {
        use pdfcraft_cos::{Dict, Object, Stream};
        let page = pdfcraft_annot::page_refs(&doc).map_err(|e| e.to_string())?.first().copied().ok_or("The mark page is missing.")?;
        let mut resources = Dict::new();
        let mut content = format!("{} {} {} rg\n", color[0], color[1], color[2]).into_bytes();
        if text == "/" {
            for (i, [x, y]) in CHECK_POINTS.iter().enumerate() {
                content.extend_from_slice(
                    format!("{} {} {}\n", 2.0 + x * symbol_size, 2.0 + y * symbol_size, if i == 0 { "m" } else { "l" }).as_bytes(),
                );
            }
            content.extend_from_slice(b"h f\n");
        } else {
            let mut dictionary = Dict::new();
            dictionary.set(b"Type".to_vec(), Object::name("Font"));
            dictionary.set(b"Subtype".to_vec(), Object::name("Type1"));
            dictionary.set(b"BaseFont".to_vec(), Object::name(font.base_font()));
            dictionary.set(b"Encoding".to_vec(), Object::name("WinAnsiEncoding"));
            let reference = doc.add(dictionary);
            let mut fonts = Dict::new();
            fonts.set(b"F1".to_vec(), Object::Ref(reference));
            resources.set(b"Font".to_vec(), Object::Dict(fonts));
            content.extend_from_slice(format!("BT /F1 {size} Tf {} {} Td\n", 4.0 + size * 0.35, 4.0 + size * 0.3).as_bytes());
            content.extend(pdfcraft_fonts::literal(&pdfcraft_fonts::win_ansi(text)));
            content.extend_from_slice(b" Tj ET\n");
        }
        let stream = doc.add(Object::Stream(Stream::flate(Dict::new(), &content)));
        doc.update_dict(page, |d| {
            d.set(b"Resources".to_vec(), Object::Dict(resources));
            d.set(b"Contents".to_vec(), Object::Ref(stream));
        })
        .map_err(|e| e.to_string())?;
    }
    let bytes = pdfcraft_cos::write_full(&doc, &pdfcraft_cos::SaveOptions::default()).map_err(|e| e.to_string())?;
    Ok(Graphic { label, width, height, file: MarkFile { name: "work-mark.pdf".into(), bytes: Arc::new(bytes), page: 0 } })
}
