//! Personal initials and a date fixed at placement, stored as one portable Stamp comment.
use pdfcraft_cos::{Dict, Object, Stream};
use pdfcraft_fonts::Std14;

pub const DATE_FORMAT: &str = "m/d/yy";
pub const HEIGHT: f64 = 24.0;

pub fn width(date: &str) -> f64 {
    4.0 + HEIGHT * 1.6 + 6.0 + Std14::Helvetica.text_width(date, 10.0)
}

pub fn build(initials: &crate::sign::Graphic, date: &str) -> Result<crate::work_marks::Graphic, String> {
    if date.is_empty() || date.len() > 32 || date.chars().any(|c| !(c.is_ascii_digit() || c == '/')) {
        return Err("Use a numeric sign-off date such as 10/15/26.".into());
    }
    let (w, h) = (width(date), HEIGHT + 4.0);
    let mut doc = pdfcraft_create::blank(w, h, 1).map_err(|e| e.to_string())?;
    let (mut content, image) = initials.draw(&mut doc, [2.0, 2.0, HEIGHT * 1.6, HEIGHT]).map_err(|e| e.to_string())?;
    let mut resources = Dict::new();
    if let Some(image) = image {
        let mut objects = Dict::new();
        objects.set(b"Im1".to_vec(), Object::Ref(image));
        resources.set(b"XObject".to_vec(), Object::Dict(objects));
    }
    let mut font = Dict::new();
    font.set(b"Type".to_vec(), Object::name("Font"));
    font.set(b"Subtype".to_vec(), Object::name("Type1"));
    font.set(b"BaseFont".to_vec(), Object::name("Helvetica"));
    font.set(b"Encoding".to_vec(), Object::name("WinAnsiEncoding"));
    let font = doc.add(font);
    let mut fonts = Dict::new();
    fonts.set(b"DateFont".to_vec(), Object::Ref(font));
    resources.set(b"Font".to_vec(), Object::Dict(fonts));
    content.push_str(&format!("\n0 g BT /DateFont 10 Tf {} {} Td ({date}) Tj ET\n", 2.0 + HEIGHT * 1.6 + 6.0, (h - 7.2) / 2.0));
    let stream = doc.add(Object::Stream(Stream::flate(Dict::new(), content.as_bytes())));
    let page = pdfcraft_annot::page_refs(&doc).map_err(|e| e.to_string())?.first().copied().ok_or("The sign-off page is missing.")?;
    doc.update_dict(page, |d| {
        d.set(b"Resources".to_vec(), Object::Dict(resources));
        d.set(b"Contents".to_vec(), Object::Ref(stream));
    })
    .map_err(|e| e.to_string())?;
    let bytes = pdfcraft_cos::write_full(&doc, &Default::default()).map_err(|e| e.to_string())?;
    Ok(crate::work_marks::Graphic::from_source(format!("Initials — {date}"), w, h, bytes))
}

impl crate::Session {
    /// Capture only the selected page region, bounded to four megapixels. Nothing is copied
    /// to the clipboard or saved until the caller accepts the resulting initials.
    pub fn capture_initials(&self, id: crate::DocId, page: usize, rect: [f64; 4]) -> Result<crate::SignatureImage, String> {
        let doc = self.get(id).ok_or("Open a document first.")?;
        if !doc.allows_copying() {
            return Err("This document does not permit copying images.".into());
        }
        let info = doc.info.pages.get(page).ok_or("That page does not exist.")?;
        let [x0, y0, x1, y1] = rect;
        if rect.iter().any(|n| !n.is_finite())
            || x0 < 0.0
            || y0 < 0.0
            || x1 > f64::from(info.width)
            || y1 > f64::from(info.height)
            || x1 - x0 < 2.0
            || y1 - y0 < 2.0
        {
            return Err("Draw a crop box of at least 2 points inside the page.".into());
        }
        let scale = (2000.0 / (x1 - x0).max(y1 - y0)).min(4.0) as f32;
        let tile = pdfcraft_render::Tile {
            x: (x0 * f64::from(scale)).floor() as u32,
            y: (y0 * f64::from(scale)).floor() as u32,
            w: ((x1 - x0) * f64::from(scale)).ceil() as u32,
            h: ((y1 - y0) * f64::from(scale)).ceil() as u32,
        };
        let config = pdfcraft_render::RenderConfig { password: doc.password.as_deref().map(std::sync::Arc::from), ..Default::default() };
        let mut renderer = pdfcraft_render::PageRenderer::new(doc.bytes.clone(), config);
        let raster = renderer.render(pdfcraft_render::RenderRequest { page, scale, tile: Some(tile), ..Default::default() });
        if let Some(error) = raster.error {
            return Err(error);
        }
        let mut image = image::RgbaImage::from_raw(raster.width, raster.height, raster.rgba.to_vec()).ok_or("Could not capture that area.")?;
        // A rendered PDF crop has a flat paper backdrop. Local photo illumination removal
        // can mistake the middle of a thick stroke for paper; remove near-white only here.
        for pixel in image.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            let keep = ((245.0 - f32::from(r.min(g).min(b))) / 35.0).clamp(0.0, 1.0);
            pixel.0[3] = (f32::from(a) * keep).round() as u8;
        }
        let bounds = image
            .enumerate_pixels()
            .filter(|(_, _, p)| p.0[3] > 16)
            .fold(None::<[u32; 4]>, |bounds, x| {
                let (x, y, _) = x;
                Some(match bounds {
                    None => [x, y, x, y],
                    Some([x0, y0, x1, y1]) => [x0.min(x), y0.min(y), x1.max(x), y1.max(y)],
                })
            })
            .ok_or("The crop is empty. Draw a box around visible initials.")?;
        let [x0, y0, x1, y1] = bounds;
        let left = x0.saturating_sub(2);
        let top = y0.saturating_sub(2);
        let right = x1.saturating_add(3).min(image.width());
        let bottom = y1.saturating_add(3).min(image.height());
        let image = image::imageops::crop_imm(&image, left, top, right - left, bottom - top).to_image();
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image).write_to(&mut png, image::ImageFormat::Png).map_err(|e| e.to_string())?;
        crate::SignatureImage::from_bytes(&png.into_inner()).map_err(|e| e.to_string())
    }
}
