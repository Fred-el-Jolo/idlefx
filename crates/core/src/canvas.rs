//! Transparent half-block canvas: two pixels per terminal cell.
//!
//! A pixel is either a color or transparent. Transparent pixels are drawn with
//! the terminal's default background, so a translucent terminal shows the
//! desktop through them. Drawing is hard-edged (no anti-aliasing): a pixel is
//! covered when its center is inside the shape. Coordinates are in pixels, as
//! floats; pixel `(x, y)` has its center at `(x + 0.5, y + 0.5)`.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

/// An opaque 24-bit color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    fn color(self) -> Color {
        Color::Rgb(self.0, self.1, self.2)
    }
}

/// A pixel: a color, or `None` for transparent.
pub type Pixel = Option<Rgb>;

/// A pixel canvas, `width` × `height` pixels, two pixels per cell vertically.
#[derive(Clone, Debug, Default)]
pub struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<Pixel>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        let mut canvas = Self::default();
        canvas.resize(width, height);
        canvas
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Change the size; reallocates only when the pixel count grows.
    pub fn resize(&mut self, width: usize, height: usize) {
        if (width, height) != (self.width, self.height) {
            self.width = width;
            self.height = height;
            self.pixels.clear();
            self.pixels.resize(width * height, None);
        }
    }

    /// Match a terminal area: one pixel per column, two per row.
    pub fn fit(&mut self, area: Rect) {
        self.resize(area.width as usize, area.height as usize * 2);
    }

    /// Make every pixel transparent.
    pub fn clear(&mut self) {
        self.pixels.fill(None);
    }

    /// Fill every pixel with one color.
    pub fn fill(&mut self, color: Rgb) {
        self.pixels.fill(Some(color));
    }

    pub fn get(&self, x: usize, y: usize) -> Pixel {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x]
        } else {
            None
        }
    }

    pub fn set(&mut self, x: usize, y: usize, pixel: Pixel) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = pixel;
        }
    }

    /// One pixel, at the pixel containing `(x, y)`.
    pub fn pixel(&mut self, x: f32, y: f32, color: Rgb) {
        if x >= 0.0 && y >= 0.0 {
            self.set(x as usize, y as usize, Some(color));
        }
    }

    /// Filled rectangle covering the pixels whose centers are in
    /// `[x0, x1) × [y0, y1)`.
    pub fn rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: Rgb) {
        let (cx0, cx1) = self.span_x(x0, x1);
        let (cy0, cy1) = self.span_y(y0, y1);
        for y in cy0..cy1 {
            for x in cx0..cx1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                if px >= x0 && px < x1 && py >= y0 && py < y1 {
                    self.pixels[y * self.width + x] = Some(color);
                }
            }
        }
    }

    /// Filled ellipse centered on `(cx, cy)` with radii `rx`, `ry`, rotated by
    /// `angle` radians (clockwise on screen, as y points down).
    #[allow(clippy::too_many_arguments)]
    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, angle: f32, color: Rgb) {
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        let reach = rx.max(ry);
        let (cx0, cx1) = self.span_x(cx - reach, cx + reach);
        let (cy0, cy1) = self.span_y(cy - reach, cy + reach);
        let (sin, cos) = angle.sin_cos();
        for y in cy0..cy1 {
            for x in cx0..cx1 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                // Rotate the offset back into the ellipse's own frame.
                let (u, v) = (dx * cos + dy * sin, -dx * sin + dy * cos);
                if (u / rx).powi(2) + (v / ry).powi(2) <= 1.0 {
                    self.pixels[y * self.width + x] = Some(color);
                }
            }
        }
    }

    /// Filled capsule: every pixel within `r` of the segment `a`–`b`.
    pub fn capsule(&mut self, ax: f32, ay: f32, bx: f32, by: f32, r: f32, color: Rgb) {
        let (cx0, cx1) = self.span_x(ax.min(bx) - r, ax.max(bx) + r);
        let (cy0, cy1) = self.span_y(ay.min(by) - r, ay.max(by) + r);
        let (sx, sy) = (bx - ax, by - ay);
        let len2 = sx * sx + sy * sy;
        for y in cy0..cy1 {
            for x in cx0..cx1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let t = if len2 > 0.0 {
                    (((px - ax) * sx + (py - ay) * sy) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (dx, dy) = (px - (ax + t * sx), py - (ay + t * sy));
                if dx * dx + dy * dy <= r * r {
                    self.pixels[y * self.width + x] = Some(color);
                }
            }
        }
    }

    /// Bounding box of the opaque pixels: `(x0, y0, x1, y1)`, exclusive end.
    pub fn bounds(&self) -> Option<(usize, usize, usize, usize)> {
        let mut b: Option<(usize, usize, usize, usize)> = None;
        for y in 0..self.height {
            for x in 0..self.width {
                if self.pixels[y * self.width + x].is_some() {
                    let (x0, y0, x1, y1) = b.unwrap_or((x, y, x + 1, y + 1));
                    b = Some((x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1)));
                }
            }
        }
        b
    }

    /// Write the pixels into a ratatui buffer, one cell per two pixels.
    ///
    /// Both transparent: a space on the default background. One transparent:
    /// `▀` or `▄` in the opaque color on the default background. Both opaque:
    /// `▀` with the top as foreground and the bottom as background.
    pub fn blit(&self, buf: &mut Buffer, area: Rect) {
        let rows = (self.height / 2).min(area.height as usize);
        let cols = self.width.min(area.width as usize);
        for cy in 0..rows {
            for x in 0..cols {
                let top = self.pixels[(cy * 2) * self.width + x];
                let bottom = self.pixels[(cy * 2 + 1) * self.width + x];
                let cell = &mut buf[(area.x + x as u16, area.y + cy as u16)];
                let (symbol, fg, bg) = match (top, bottom) {
                    (None, None) => (" ", Color::Reset, Color::Reset),
                    (Some(t), None) => ("▀", t.color(), Color::Reset),
                    (None, Some(b)) => ("▄", b.color(), Color::Reset),
                    (Some(t), Some(b)) => ("▀", t.color(), b.color()),
                };
                cell.set_symbol(symbol).set_fg(fg).set_bg(bg);
            }
        }
    }

    fn span_x(&self, lo: f32, hi: f32) -> (usize, usize) {
        span(lo, hi, self.width)
    }

    fn span_y(&self, lo: f32, hi: f32) -> (usize, usize) {
        span(lo, hi, self.height)
    }
}

/// The pixel index range that can hold centers in `[lo, hi]`, clipped.
fn span(lo: f32, hi: f32, max: usize) -> (usize, usize) {
    let a = (lo - 0.5).floor().max(0.0) as usize;
    let b = ((hi + 0.5).ceil().max(0.0) as usize).min(max);
    (a.min(max), b)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgb = Rgb(255, 0, 0);
    const BLUE: Rgb = Rgb(0, 0, 255);

    fn blit_one(top: Pixel, bottom: Pixel) -> (String, Color, Color) {
        let mut c = Canvas::new(1, 2);
        c.set(0, 0, top);
        c.set(0, 1, bottom);
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        c.blit(&mut buf, area);
        let cell = &buf[(0, 0)];
        (cell.symbol().to_string(), cell.fg, cell.bg)
    }

    #[test]
    fn cell_both_transparent() {
        assert_eq!(blit_one(None, None), (" ".into(), Color::Reset, Color::Reset));
    }

    #[test]
    fn cell_top_only() {
        assert_eq!(
            blit_one(Some(RED), None),
            ("▀".into(), Color::Rgb(255, 0, 0), Color::Reset)
        );
    }

    #[test]
    fn cell_bottom_only() {
        assert_eq!(
            blit_one(None, Some(BLUE)),
            ("▄".into(), Color::Rgb(0, 0, 255), Color::Reset)
        );
    }

    #[test]
    fn cell_both_opaque() {
        assert_eq!(
            blit_one(Some(RED), Some(BLUE)),
            ("▀".into(), Color::Rgb(255, 0, 0), Color::Rgb(0, 0, 255))
        );
    }

    /// Render the canvas as rows of `#` (opaque) and `.` (transparent).
    fn map(c: &Canvas) -> Vec<String> {
        (0..c.height())
            .map(|y| {
                (0..c.width())
                    .map(|x| if c.get(x, y).is_some() { '#' } else { '.' })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn prim_pixel() {
        let mut c = Canvas::new(3, 3);
        c.pixel(1.7, 2.2, RED);
        c.pixel(-1.0, 0.0, RED); // off canvas: ignored
        c.pixel(5.0, 0.0, RED);
        assert_eq!(map(&c), ["...", "...", ".#."]);
    }

    #[test]
    fn prim_rect() {
        let mut c = Canvas::new(5, 4);
        c.rect(1.0, 1.0, 4.0, 3.0, RED);
        assert_eq!(map(&c), [".....", ".###.", ".###.", "....."]);
    }

    #[test]
    fn prim_ellipse() {
        let mut c = Canvas::new(7, 5);
        c.ellipse(3.5, 2.5, 3.0, 2.0, 0.0, RED);
        assert_eq!(
            map(&c),
            ["...#...", ".#####.", "#######", ".#####.", "...#..."]
        );
    }

    #[test]
    fn prim_ellipse_rotated() {
        // A 90° rotation swaps the radii.
        let mut a = Canvas::new(7, 7);
        a.ellipse(3.5, 3.5, 2.6, 1.2, std::f32::consts::FRAC_PI_2, RED);
        let mut b = Canvas::new(7, 7);
        b.ellipse(3.5, 3.5, 1.2, 2.6, 0.0, RED);
        assert_eq!(map(&a), map(&b));
    }

    #[test]
    fn prim_capsule() {
        let mut c = Canvas::new(7, 3);
        c.capsule(1.5, 1.5, 5.5, 1.5, 1.0, RED);
        assert_eq!(map(&c), [".#####.", "#######", ".#####."]);
    }

    #[test]
    fn prim_clips_at_edges() {
        let mut c = Canvas::new(4, 4);
        c.ellipse(0.0, 0.0, 3.0, 3.0, 0.0, RED);
        c.capsule(-5.0, 3.5, 10.0, 3.5, 0.5, BLUE);
        assert_eq!(map(&c), ["###.", "###.", "##..", "####"]);
    }

    #[test]
    fn bounds_of_opaque_pixels() {
        let mut c = Canvas::new(6, 6);
        assert_eq!(c.bounds(), None);
        c.rect(1.0, 2.0, 4.0, 5.0, RED);
        assert_eq!(c.bounds(), Some((1, 2, 4, 5)));
    }
}
