//! The product's name and logo, for a build that carries SolidCraft inside another product.
//!
//! The default is SolidCraft with its own mark, drawn in code (`icons::glyphs::brand_mark`), and nothing here
//! changes it. A host sets a name and, optionally, a PNG logo once at start; the application bar, the About
//! window and the start page's About entry then use them. The state belongs to the UI thread.

use std::cell::{Cell, RefCell};

/// The name SolidCraft ships under; while it is the name in use, every label stays as the catalogs have it.
pub const DEFAULT_NAME: &str = "SolidCraft";
const MAX_NAME_CHARS: usize = 24;
const MAX_LOGO_BYTES: usize = 1 << 20;

struct Custom {
    name: String,
    logo: Option<egui::ColorImage>,
}

thread_local! {
    static CUSTOM: RefCell<Option<Custom>> = const { RefCell::new(None) };
    /// Bumped by every `set`/`reset`, so a texture made from an earlier logo is never drawn again.
    static GENERATION: Cell<u64> = const { Cell::new(0) };
    static TEXTURE: RefCell<Option<(u64, egui::TextureHandle)>> = const { RefCell::new(None) };
}

/// Use `name` and, when it decodes as a PNG of at most 1 MiB, `logo_png`. An empty or over-long name keeps the
/// default; a logo that does not decode keeps the default mark. Each part is judged alone.
pub fn set(name: &str, logo_png: Option<&[u8]>) {
    let name = name.trim();
    let name = if name.is_empty() || name.chars().count() > MAX_NAME_CHARS { DEFAULT_NAME } else { name };
    let logo = logo_png.filter(|b| b.len() <= MAX_LOGO_BYTES).and_then(decode);
    CUSTOM.with(|c| *c.borrow_mut() = Some(Custom { name: name.to_string(), logo }));
    GENERATION.with(|g| g.set(g.get() + 1));
}

/// Back to SolidCraft's own name and mark.
pub fn reset() {
    CUSTOM.with(|c| *c.borrow_mut() = None);
    GENERATION.with(|g| g.set(g.get() + 1));
}

fn decode(png: &[u8]) -> Option<egui::ColorImage> {
    let rgba = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?.to_rgba8();
    let (w, h) = rgba.dimensions();
    (w > 0 && h > 0 && w <= 2048 && h <= 2048).then(|| egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw()))
}

pub fn name() -> String {
    CUSTOM.with(|c| c.borrow().as_ref().map_or_else(|| DEFAULT_NAME.to_string(), |c| c.name.clone()))
}

/// True once a host has given a name other than SolidCraft, or a logo.
pub fn is_custom() -> bool {
    CUSTOM.with(|c| c.borrow().as_ref().is_some_and(|c| c.name != DEFAULT_NAME || c.logo.is_some()))
}

/// "About <name>": SolidCraft's own catalog entry (all languages) under its own name, the sentence template otherwise.
pub fn about_title() -> String {
    let name = name();
    if name == DEFAULT_NAME {
        crate::i18n::tr_label("About SolidCraft").to_string()
    } else {
        crate::i18n::format("brand.about", &[("name", &name)])
    }
}

/// Draw the host's logo about `height` px tall, centred on `c`. False when there is none: draw the default mark.
pub fn paint_logo(ctx: &egui::Context, painter: &egui::Painter, c: egui::Pos2, height: f32) -> bool {
    let generation = GENERATION.with(Cell::get);
    let Some((size, texture)) = TEXTURE.with(|t| {
        let mut t = t.borrow_mut();
        if t.as_ref().is_none_or(|(g, _)| *g != generation) {
            *t = CUSTOM.with(|c| {
                let c = c.borrow();
                let image = c.as_ref()?.logo.as_ref()?;
                Some((generation, ctx.load_texture("brand_logo", image.clone(), egui::TextureOptions::LINEAR)))
            });
        }
        t.as_ref().map(|(_, h)| (h.size_vec2(), h.clone()))
    }) else {
        return false;
    };
    let width = height * size.x / size.y;
    let rect = egui::Rect::from_center_size(c, egui::vec2(width, height));
    painter.image(texture.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
    true
}

/// How far right of the bar's left edge the product name starts, and how wide it may be before the Home button.
pub fn name_width(painter: &egui::Painter, font: egui::FontId, colour: egui::Color32) -> f32 {
    painter.layout_no_wrap(name(), font, colour).size().x
}
