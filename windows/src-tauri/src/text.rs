//! Gömülü JetBrains Mono ile metin çizimi (yerel pencereler için: balon, görev çubuğu çipi).
//! ab_glyph harfleri yalnızca çizilirken işler; yazı tipinin tamamı belleğe açılmaz.

use crate::img::Img;
use ab_glyph::{point, Font, FontRef, FontVec, GlyphId};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Weight { Regular, Medium, SemiBold, Bold, ExtraBold }

fn font(w: Weight) -> &'static FontRef<'static> {
    static F: [OnceLock<FontRef<'static>>; 5] = [OnceLock::new(), OnceLock::new(), OnceLock::new(), OnceLock::new(), OnceLock::new()];
    F[w as usize].get_or_init(|| {
        let b: &'static [u8] = match w {
            Weight::Regular => include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf"),
            Weight::Medium => include_bytes!("../../assets/fonts/JetBrainsMono-Medium.ttf"),
            Weight::SemiBold => include_bytes!("../../assets/fonts/JetBrainsMono-SemiBold.ttf"),
            Weight::Bold => include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf"),
            Weight::ExtraBold => include_bytes!("../../assets/fonts/JetBrainsMono-ExtraBold.ttf"),
        };
        FontRef::try_from_slice(b).expect("font")
    })
}

/// JetBrains Mono'da olmayan işaretler (✎, ▤, ⚙ …) için Windows'un sembol yazı tipi (Mac'te sistem yedeği gibi).
fn fallback() -> Option<&'static FontVec> {
    static F: OnceLock<Option<FontVec>> = OnceLock::new();
    F.get_or_init(|| {
        let dir = std::env::var_os("WINDIR").map(std::path::PathBuf::from).unwrap_or_else(|| "C:\\Windows".into()).join("Fonts");
        ["seguisym.ttf", "segoeui.ttf"].iter()
            .find_map(|n| std::fs::read(dir.join(n)).ok())
            .and_then(|b| FontVec::try_from_vec(b).ok())
    }).as_ref()
}

enum Pick { Main(&'static FontRef<'static>), Fb(&'static FontVec) }

fn pick(c: char, w: Weight) -> Pick {
    let f = font(w);
    if f.glyph_id(c).0 != 0 || c.is_whitespace() { return Pick::Main(f); }
    match fallback() { Some(fb) if fb.glyph_id(c).0 != 0 => Pick::Fb(fb), _ => Pick::Main(f) }
}

/// Punto (em) boyutunu ab_glyph ölçeğine çevirir (ab_glyph ascent−descent yüksekliğini ölçekler).
fn px_scale<F: Font>(f: &F, size: f32) -> f32 {
    let upem = f.units_per_em().unwrap_or(1000.0);
    size * (f.ascent_unscaled() - f.descent_unscaled()) / upem
}

fn advance<F: Font>(f: &F, c: char, size: f32) -> f32 {
    let upem = f.units_per_em().unwrap_or(1000.0);
    f.h_advance_unscaled(f.glyph_id(c)) * size / upem
}

fn draw_glyph<F: Font>(f: &F, img: &mut Img, id: GlyphId, x: f32, baseline: f32, size: f32, rgb: (u8, u8, u8), alpha: f32) {
    let g = id.with_scale_and_position(px_scale(f, size), point(x, baseline));
    if let Some(o) = f.outline_glyph(g) {
        let b = o.px_bounds();
        o.draw(|gx, gy, cov| {
            let a = cov * alpha;
            if a > 0.0 {
                img.over(b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32, crate::img::rgba(rgb.0, rgb.1, rgb.2, a));
            }
        });
    }
}

/// Satırın genişliği (px).
pub fn width(text: &str, size: f32, w: Weight) -> f32 {
    text.chars().map(|c| match pick(c, w) { Pick::Main(f) => advance(f, c, size), Pick::Fb(f) => advance(f, c, size) }).sum()
}

/// Satır yüksekliği bilgileri (px): (ascent, descent pozitif, satır yüksekliği).
pub fn line_metrics(size: f32, w: Weight) -> (f32, f32, f32) {
    let f = font(w);
    let upem = f.units_per_em().unwrap_or(1000.0);
    let a = f.ascent_unscaled() * size / upem;
    let d = -f.descent_unscaled() * size / upem;
    (a, d, a + d)
}

/// `x` soldan, `baseline` taban çizgisi (px). Renk önceden çarpılmamış RGB + saydamlık.
pub fn draw(img: &mut Img, text: &str, x: f32, baseline: f32, size: f32, w: Weight, rgb: (u8, u8, u8), alpha: f32) {
    let mut pen = x;
    for c in text.chars() {
        match pick(c, w) {
            Pick::Main(f) => { draw_glyph(f, img, f.glyph_id(c), pen, baseline, size, rgb, alpha); pen += advance(f, c, size); }
            Pick::Fb(f) => { draw_glyph(f, img, f.glyph_id(c), pen, baseline, size, rgb, alpha); pen += advance(f, c, size); }
        }
    }
}
