use super::brand;

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::new();
    image::RgbaImage::from_pixel(w, h, image::Rgba([10, 20, 200, 255]))
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .expect("png");
    out
}

#[test]
fn without_a_host_the_product_is_solidcraft_with_its_own_mark() {
    brand::reset();
    assert_eq!(brand::name(), "SolidCraft");
    assert!(!brand::is_custom());
    assert_eq!(brand::about_title(), "About SolidCraft");
}

#[test]
fn a_host_name_and_logo_replace_the_defaults_and_reset_restores_them() {
    brand::set("SolidZ", Some(&png(8, 4)));
    assert_eq!(brand::name(), "SolidZ");
    assert!(brand::is_custom());
    assert_eq!(brand::about_title(), "About SolidZ");
    brand::reset();
    assert_eq!(brand::name(), "SolidCraft");
}

#[test]
fn a_bad_name_or_logo_is_ignored_part_by_part() {
    brand::set("  ", Some(b"not a png"));
    assert_eq!(brand::name(), "SolidCraft");
    assert!(!brand::is_custom(), "neither part was usable");
    brand::set(&"x".repeat(25), None);
    assert_eq!(brand::name(), "SolidCraft", "too long");
    brand::set("SolidZ", Some(&vec![0u8; (1 << 20) + 1]));
    assert_eq!(brand::name(), "SolidZ", "the name stands when only the logo is refused");
    brand::reset();
}

#[test]
fn the_logo_is_drawn_only_when_the_host_gave_one() {
    let ctx = egui::Context::default();
    let mut drawn = Vec::new();
    for logo in [None, Some(png(8, 4))] {
        brand::set("SolidZ", logo.as_deref());
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            drawn.push(brand::paint_logo(ui.ctx(), ui.painter(), egui::pos2(20.0, 17.0), 20.0));
        });
    }
    assert_eq!(drawn, [false, true]);
    brand::reset();
}
