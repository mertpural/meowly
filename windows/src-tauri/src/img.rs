//! Küçük, bağımlılıksız görüntü tamponu. Pikseller önceden çarpılmış (premultiplied) BGRA:
//! UpdateLayeredWindow doğrudan bu biçimi ister.

#[derive(Clone)]
pub struct Img {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[u8; 4]>, // B, G, R, A (premultiplied)
}

impl Img {
    pub fn new(w: usize, h: usize) -> Img {
        Img { w, h, px: vec![[0; 4]; w * h] }
    }

    /// PNG'yi çözüp önceden çarpılmış BGRA'ya çevirir.
    pub fn decode_png(bytes: &[u8]) -> Img {
        let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
        dec.set_transformations(png::Transformations::normalize_to_color8());
        let mut r = dec.read_info().expect("png");
        let mut buf = vec![0; r.output_buffer_size()];
        let info = r.next_frame(&mut buf).expect("png frame");
        let (w, h) = (info.width as usize, info.height as usize);
        let mut out = Img::new(w, h);
        for i in 0..w * h {
            let (rr, gg, bb, aa) = match info.color_type {
                png::ColorType::Rgba => (buf[i * 4], buf[i * 4 + 1], buf[i * 4 + 2], buf[i * 4 + 3]),
                png::ColorType::Rgb => (buf[i * 3], buf[i * 3 + 1], buf[i * 3 + 2], 255),
                png::ColorType::GrayscaleAlpha => (buf[i * 2], buf[i * 2], buf[i * 2], buf[i * 2 + 1]),
                _ => (buf[i], buf[i], buf[i], 255),
            };
            let a = aa as u32;
            let pm = |c: u8| ((c as u32 * a + 127) / 255) as u8;
            out.px[i] = [pm(bb), pm(gg), pm(rr), aa];
        }
        out
    }

    /// Alan ortalamalı yeniden boyutlandırma (küçültmede bulanık değil, her kaynak pikseli payı kadar sayılır).
    /// Büyütmede en yakın komşu: piksel sanat keskin kalır.
    pub fn resample(&self, scale: f64) -> Img {
        let nw = ((self.w as f64 * scale).round() as usize).max(1);
        let nh = ((self.h as f64 * scale).round() as usize).max(1);
        if scale >= 1.0 {
            let mut o = Img::new(nw, nh);
            for y in 0..nh {
                let sy = ((y as f64 + 0.5) / scale) as usize;
                for x in 0..nw {
                    let sx = ((x as f64 + 0.5) / scale) as usize;
                    o.px[y * nw + x] = self.px[sy.min(self.h - 1) * self.w + sx.min(self.w - 1)];
                }
            }
            return o;
        }
        let sx = self.w as f64 / nw as f64;
        let sy = self.h as f64 / nh as f64;
        // Yatay geçiş
        let mut tmp = vec![[0f32; 4]; nw * self.h];
        for y in 0..self.h {
            for x in 0..nw {
                let (a, b) = (x as f64 * sx, (x + 1) as f64 * sx);
                let mut acc = [0f32; 4];
                let mut i = a.floor() as usize;
                while (i as f64) < b && i < self.w {
                    let wgt = ((i + 1) as f64).min(b) - (i as f64).max(a);
                    let p = self.px[y * self.w + i];
                    for c in 0..4 { acc[c] += p[c] as f32 * wgt as f32; }
                    i += 1;
                }
                for c in 0..4 { acc[c] /= sx as f32; }
                tmp[y * nw + x] = acc;
            }
        }
        // Dikey geçiş
        let mut o = Img::new(nw, nh);
        for y in 0..nh {
            let (a, b) = (y as f64 * sy, (y + 1) as f64 * sy);
            for x in 0..nw {
                let mut acc = [0f32; 4];
                let mut i = a.floor() as usize;
                while (i as f64) < b && i < self.h {
                    let wgt = ((i + 1) as f64).min(b) - (i as f64).max(a);
                    let p = tmp[i * nw + x];
                    for c in 0..4 { acc[c] += p[c] * wgt as f32; }
                    i += 1;
                }
                let px = &mut o.px[y * nw + x];
                for c in 0..4 { px[c] = (acc[c] / sy as f32).round().clamp(0.0, 255.0) as u8; }
            }
        }
        o
    }

    #[inline]
    pub fn over(&mut self, x: i32, y: i32, c: [u8; 4]) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h || c[3] == 0 { return; }
        let d = &mut self.px[y as usize * self.w + x as usize];
        if c[3] == 255 { *d = c; return; }
        let ia = 255 - c[3] as u32;
        for i in 0..4 { d[i] = (c[i] as u32 + (d[i] as u32 * ia + 127) / 255).min(255) as u8; }
    }

    /// Kaynağın [sy0, sy0+sh) satırlarını hedefe çizer; dikeyde `dh` yüksekliğe en yakın komşu ile uzatır.
    /// `shift(row)`: satır başına ek yatay kayma (bükülme için).
    pub fn blit_rows(&mut self, src: &Img, sy0: usize, sh: usize, dx: i32, dy: i32, dh: usize, alpha: f32, shift: &dyn Fn(usize) -> i32) {
        if sh == 0 || dh == 0 { return; }
        for oy in 0..dh {
            let ry = sy0 + ((oy as f64 + 0.5) * sh as f64 / dh as f64) as usize;
            let ry = ry.min(sy0 + sh - 1).min(src.h - 1);
            let off = shift(ry);
            let ty = dy + oy as i32;
            if ty < 0 || ty as usize >= self.h { continue; }
            for x in 0..src.w {
                let mut p = src.px[ry * src.w + x];
                if p[3] == 0 { continue; }
                if alpha < 1.0 { for c in 0..4 { p[c] = (p[c] as f32 * alpha).round() as u8; } }
                self.over(dx + x as i32 + off, ty, p);
            }
        }
    }

    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 4]) {
        for yy in y.max(0)..(y + h).min(self.h as i32) {
            for xx in x.max(0)..(x + w).min(self.w as i32) {
                self.over(xx, yy, c);
            }
        }
    }

    /// Kesirli kenarlı dikdörtgen (kenar pikselleri kapladığı oranda boyanır).
    pub fn fill_aa(&mut self, x: f64, y: f64, w: f64, h: f64, rgb: (u8, u8, u8), a: f32) {
        if w <= 0.0 || h <= 0.0 { return; }
        let (x1, y1) = (x + w, y + h);
        for py in y.floor().max(0.0) as i32..(y1.ceil() as i32).min(self.h as i32) {
            let cy = (y1.min(py as f64 + 1.0) - y.max(py as f64)).max(0.0);
            for px in x.floor().max(0.0) as i32..(x1.ceil() as i32).min(self.w as i32) {
                let cx = (x1.min(px as f64 + 1.0) - x.max(px as f64)).max(0.0);
                let cov = (cx * cy) as f32 * a;
                if cov > 0.0 { self.over(px, py, rgba(rgb.0, rgb.1, rgb.2, cov)); }
            }
        }
    }

    /// Kenarları yumuşatılmış daire/halka: merkez (cx, cy), yarıçap r, kalınlık `width` (0 = dolu).
    /// `frac` < 1 ise yalnızca üstten saat yönünde o orandaki yay çizilir (yuvarlak uçlu).
    pub fn circle(&mut self, cx: f64, cy: f64, r: f64, width: f64, frac: f64, rgb: (u8, u8, u8), a: f32) {
        let ext = r + width + 1.0;
        for py in (cy - ext).floor() as i32..=(cy + ext).ceil() as i32 {
            for px in (cx - ext).floor() as i32..=(cx + ext).ceil() as i32 {
                let (dx, dy) = (px as f64 + 0.5 - cx, py as f64 + 0.5 - cy);
                let d = (dx * dx + dy * dy).sqrt();
                let mut cov = if width <= 0.0 { (r - d + 0.5).clamp(0.0, 1.0) } else { (width / 2.0 - (d - r).abs() + 0.5).clamp(0.0, 1.0) };
                if frac < 1.0 && cov > 0.0 {
                    // Açı: üstten (12 yönü) saat yönünde 0…1
                    let ang = (dx.atan2(-dy) / std::f64::consts::TAU).rem_euclid(1.0);
                    if ang > frac {
                        // Yuvarlak uç: başlangıç ve bitiş noktalarına uzaklık
                        let end = |t: f64| { let th = t * std::f64::consts::TAU; (cx + r * th.sin(), cy - r * th.cos()) };
                        let near = [end(0.0), end(frac)].iter()
                            .map(|(ex, ey)| (width / 2.0 - ((px as f64 + 0.5 - ex).powi(2) + (py as f64 + 0.5 - ey).powi(2)).sqrt() + 0.5).clamp(0.0, 1.0))
                            .fold(0.0, f64::max);
                        cov = near;
                    }
                }
                if cov > 0.0 { self.over(px, py, rgba(rgb.0, rgb.1, rgb.2, cov as f32 * a)); }
            }
        }
    }

    /// PNG baytları (önceden çarpmayı geri alır).
    pub fn png_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut e = png::Encoder::new(&mut out, self.w as u32, self.h as u32);
            e.set_color(png::ColorType::Rgba);
            e.set_depth(png::BitDepth::Eight);
            if let Ok(mut wr) = e.write_header() {
                let mut data = Vec::with_capacity(self.w * self.h * 4);
                for p in &self.px {
                    let a = p[3] as u32;
                    let un = |c: u8| if a == 0 { 0 } else { ((c as u32 * 255 + a / 2) / a).min(255) as u8 };
                    data.extend_from_slice(&[un(p[2]), un(p[1]), un(p[0]), p[3]]);
                }
                let _ = wr.write_image_data(&data);
            }
        }
        out
    }

    /// Geliştirme: tamponu PNG olarak yazar (önceden çarpmayı geri alır).
    pub fn save_png(&self, path: &std::path::Path) {
        let Ok(f) = std::fs::File::create(path) else { return };
        let mut e = png::Encoder::new(std::io::BufWriter::new(f), self.w as u32, self.h as u32);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        let Ok(mut wr) = e.write_header() else { return };
        let mut data = Vec::with_capacity(self.w * self.h * 4);
        for p in &self.px {
            let a = p[3] as u32;
            let un = |c: u8| if a == 0 { 0 } else { ((c as u32 * 255 + a / 2) / a).min(255) as u8 };
            data.extend_from_slice(&[un(p[2]), un(p[1]), un(p[0]), p[3]]);
        }
        let _ = wr.write_image_data(&data);
    }
}

/// sRGB rengi (0–255) + saydamlık (0–1) → önceden çarpılmış BGRA.
pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> [u8; 4] {
    let a8 = (a.clamp(0.0, 1.0) * 255.0).round() as u32;
    let pm = |c: u8| ((c as u32 * a8 + 127) / 255) as u8;
    [pm(b), pm(g), pm(r), a8 as u8]
}
