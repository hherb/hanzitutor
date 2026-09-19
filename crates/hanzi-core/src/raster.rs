//! Raster ink: how much paper a stroke actually covers.
//!
//! Stroke centre-lines say *where* a stroke went; they say nothing about how
//! much ink was put down. A trace that follows the right path but is drawn far
//! too thin, or that overshoots wildly, is indistinguishable from a careful one
//! to a centre-line metric — the shape score is deliberately scale-invariant, so
//! it cannot see width at all. This module answers the other question by
//! rasterising both sides and comparing them as sets of inked cells:
//!
//! * the **user's strokes** become round-capped pen strokes of the width the
//!   interface actually draws ([`INK_WIDTH`]);
//! * the **reference** becomes the filled stroke outline — the same paths the
//!   canvas fills as the faint guide, so the comparison is against the ink the
//!   learner is being shown.
//!
//! The grid is deliberately coarse: [`GRID`] cells across the 1024-unit box is 4
//! design units per cell, which is fine enough to separate a correct trace from
//! a thin one and coarse enough to keep a grade in the low milliseconds.
//!
//! ## Why the measure is area, not intersection-over-union
//!
//! Intersection-over-union between the attempt's ink and the glyph outline looks
//! like the obvious measure, and it is the wrong one, for two reasons that were
//! both measured on the real dataset rather than assumed.
//!
//! First, a perfect trace is not the same *shape* as the outline: the pen has
//! one width, while the outline is calligraphic, tapered and wider on average
//! (the median reference stroke is about 46 units against a 36-unit pen). Raw
//! IoU therefore tops out near 0.7 for a flawless attempt, so "perfect" would be
//! unreachable and every figure would depend on how fat the font happens to be.
//!
//! Second, and worse, IoU falls when a *correctly sized* band lands in slightly
//! the wrong place — and placement is already measured, with its own generous
//! tolerance, by [`crate::grade`]. Jittering a right-width trace by 3% of the box
//! (the "sloppy" row of `selfcheck`) keeps its ink area but drops the IoU ratio
//! to 0.47, which marked a sloppy hand illegible 96% of the time. That is a
//! double-counted placement fault, not an ink fault.
//!
//! So the ink measure is split in two, which is what the two halves were always
//! about:
//!
//! * **amount** — [`Ink::amount_agreement`], the attempt's inked area against
//!   what a correct trace at the nominal pen width would put down. Blind to
//!   where the ink went; `1.0` for a correct trace, about `0.33` for a pen a
//!   third of the width, and the same again for a wild overshoot.
//! * **coverage** — how much of the glyph outline the attempt reached, which is
//!   the "you never drew that part" signal and the one place the filled outline
//!   is the reference rather than the ideal pen stroke.

use crate::geom::{Point, EM, FONT_TOP_Y};

/// Cells across the character box. 256 gives 4 design units per cell.
pub const GRID: usize = 256;

/// Width of the user's ink, in design units, out of 1024.
///
/// This is what the canvas draws a stroke with (`INK_WIDTH` in
/// `src/lib/render.ts`), and the interface passes its own value through
/// [`crate::GradeOptions::ink_width`] so the two cannot drift apart.
pub const INK_WIDTH: f32 = 36.0;

/// Design units per raster cell.
const CELL: f32 = EM / GRID as f32;

/// 64-bit words in one mask.
const WORDS: usize = GRID * GRID / 64;

/// Longest chord, in design units, used to flatten a curve.
///
/// Half a cell would be enough; a whole one keeps the loop counts small while
/// still resolving a curve far better than the grid can show.
const FLATTEN_CHORD: f32 = CELL;

/// A rasterised set of inked cells, as a bitset.
#[derive(Clone)]
pub(crate) struct Ink {
    bits: Vec<u64>,
    count: u32,
}

impl Ink {
    pub(crate) fn empty() -> Self {
        Self {
            bits: vec![0; WORDS],
            count: 0,
        }
    }

    fn set(&mut self, px: usize, py: usize) {
        debug_assert!(px < GRID && py < GRID);
        let index = py * GRID + px;
        let word = &mut self.bits[index / 64];
        let bit = 1u64 << (index % 64);
        if *word & bit == 0 {
            *word |= bit;
            self.count += 1;
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Add every inked cell of `other` to this mask.
    pub(crate) fn or_with(&mut self, other: &Ink) {
        for (a, b) in self.bits.iter_mut().zip(&other.bits) {
            *a |= *b;
        }
        self.count = self.bits.iter().map(|w| w.count_ones()).sum();
    }

    pub(crate) fn intersection_count(&self, other: &Ink) -> u32 {
        self.bits
            .iter()
            .zip(&other.bits)
            .map(|(a, b)| (a & b).count_ones())
            .sum()
    }

    /// Agreement between how much ink two masks hold, `0..=1`.
    ///
    /// The *area* ratio, not intersection-over-union, and deliberately so: IoU
    /// also falls when a correctly-sized band lands in slightly the wrong place,
    /// which is a placement fault that [`crate::grade`] already measures with
    /// its own tolerance. Using IoU here double-counts a wobbly hand — measured
    /// on the real dataset, a 3%-jitter trace of the right width keeps its ink
    /// area but drops to an IoU ratio of 0.47, which would have marked a sloppy
    /// hand illegible 96% of the time. Area answers only the question this
    /// measure exists for: how much ink did you put down.
    pub(crate) fn amount_agreement(&self, other: &Ink) -> f32 {
        let (small, large) = (self.count.min(other.count), self.count.max(other.count));
        if large == 0 {
            // Two empty masks: nothing was drawn, which is a perfect match for
            // "draw nothing" and the caller decides what that is worth.
            return 1.0;
        }
        small as f32 / large as f32
    }

    /// Rasterise the stored outline paths, in font space, as filled ink.
    ///
    /// Returns an empty mask when nothing could be parsed, which the caller
    /// treats as "no outline available" rather than as a fault: a missing
    /// outline must not make a correct attempt unscoreable.
    pub(crate) fn from_outlines(outlines: &[&str]) -> Ink {
        let mut ink = Ink::empty();
        for path in outlines {
            for polygon in flatten_path(path) {
                ink.fill_polygon(&polygon);
            }
        }
        ink
    }

    /// Rasterise polylines, in display space, as round-capped pen strokes.
    ///
    /// The union of one capsule per segment is exactly what stroking a polyline
    /// with round caps and joins paints, so joins need no special case.
    pub(crate) fn from_strokes(strokes: &[Vec<Point>], width: f32) -> Ink {
        let mut ink = Ink::empty();
        let r = (width.max(0.0) / 2.0) / CELL;
        let rr = r * r;
        for stroke in strokes {
            let grid: Vec<Point> = stroke
                .iter()
                .map(|p| Point::new(p.x / CELL, p.y / CELL))
                .collect();
            match grid.len() {
                0 => {}
                // A single tap paints the dot the canvas draws for it.
                1 => ink.fill_disc(grid[0], rr),
                _ => {
                    for w in grid.windows(2) {
                        ink.fill_capsule(w[0], w[1], r, rr);
                    }
                }
            }
        }
        ink
    }

    // ---- rasterisation ----------------------------------------------------

    /// Scanline-fill one closed polygon given in grid coordinates.
    ///
    /// Every outline in the dataset is a single simple closed subpath, so the
    /// even-odd pairing of crossings below is also the fill the canvas's
    /// non-zero rule produces.
    fn fill_polygon(&mut self, polygon: &[Point]) {
        if polygon.len() < 3 {
            return;
        }
        let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
        for p in polygon {
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        let first = (min_y.floor().max(0.0)) as usize;
        let last = (max_y.ceil().min((GRID - 1) as f32)) as usize;

        let mut crossings: Vec<f32> = Vec::with_capacity(polygon.len());
        for row in first..=last {
            let yc = row as f32 + 0.5;
            crossings.clear();
            for i in 0..polygon.len() {
                let a = polygon[i];
                let b = polygon[(i + 1) % polygon.len()];
                // Half-open test: a vertex is counted once, by the edge above it.
                if (a.y <= yc) != (b.y <= yc) {
                    let t = (yc - a.y) / (b.y - a.y);
                    crossings.push(a.x + (b.x - a.x) * t);
                }
            }
            if crossings.len() < 2 {
                continue;
            }
            crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            for pair in crossings.as_chunks::<2>().0 {
                self.fill_span(row, pair[0], pair[1]);
            }
        }
    }

    /// Fill the cells of one row whose centres lie in `[x0, x1)`.
    fn fill_span(&mut self, row: usize, x0: f32, x1: f32) {
        let start = (x0 - 0.5).ceil().max(0.0) as usize;
        let end = (x1 - 0.5).ceil();
        if end <= 0.0 {
            return;
        }
        let end = (end as usize).min(GRID);
        for px in start..end {
            self.set(px, row);
        }
    }

    fn fill_disc(&mut self, centre: Point, rr: f32) {
        let x0 = (centre.x - rr.sqrt()).floor().max(0.0) as usize;
        let x1 = (centre.x + rr.sqrt()).ceil().min((GRID - 1) as f32) as usize;
        let y0 = (centre.y - rr.sqrt()).floor().max(0.0) as usize;
        let y1 = (centre.y + rr.sqrt()).ceil().min((GRID - 1) as f32) as usize;
        for py in y0..=y1 {
            for px in x0..=x1 {
                let dx = px as f32 + 0.5 - centre.x;
                let dy = py as f32 + 0.5 - centre.y;
                if dx * dx + dy * dy <= rr {
                    self.set(px, py);
                }
            }
        }
    }

    fn fill_capsule(&mut self, a: Point, b: Point, r: f32, rr: f32) {
        let x0 = (a.x.min(b.x) - r).floor().max(0.0) as usize;
        let x1 = (a.x.max(b.x) + r).ceil().min((GRID - 1) as f32) as usize;
        let y0 = (a.y.min(b.y) - r).floor().max(0.0) as usize;
        let y1 = (a.y.max(b.y) + r).ceil().min((GRID - 1) as f32) as usize;
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len2 = dx * dx + dy * dy;
        for py in y0..=y1 {
            let cy = py as f32 + 0.5;
            for px in x0..=x1 {
                let cx = px as f32 + 0.5;
                let t = if len2 <= f32::EPSILON {
                    0.0
                } else {
                    (((cx - a.x) * dx + (cy - a.y) * dy) / len2).clamp(0.0, 1.0)
                };
                let qx = a.x + dx * t;
                let qy = a.y + dy * t;
                let (ex, ey) = (cx - qx, cy - qy);
                if ex * ex + ey * ey <= rr {
                    self.set(px, py);
                }
            }
        }
    }
}

// ---- SVG path parsing ------------------------------------------------------

/// One token of an SVG path: a command letter or a number.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Token {
    Command(u8),
    Number(f32),
}

/// Split path data into command letters and numbers.
///
/// Make Me a Hanzi emits only absolute `M`, `L`, `Q`, `C` and `Z`, which is what
/// this understands. Anything else makes the whole path unparseable, and the
/// caller then simply has no outline to compare against.
fn tokenize(path: &str) -> Option<Vec<Token>> {
    let bytes = path.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() || b == b',' {
            i += 1;
        } else if b.is_ascii_alphabetic() {
            tokens.push(Token::Command(b));
            i += 1;
        } else {
            let start = i;
            if bytes[i] == b'+' || bytes[i] == b'-' {
                i += 1;
            }
            let mut digits = false;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
                digits = true;
            }
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                    digits = true;
                }
            }
            if !digits {
                return None;
            }
            if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
                i += 1;
                if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                    i += 1;
                }
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let text = std::str::from_utf8(&bytes[start..i]).ok()?;
            tokens.push(Token::Number(text.parse::<f32>().ok()?));
        }
    }
    Some(tokens)
}

/// A font-space point as a grid-space point in *display* orientation.
fn to_grid(x: f32, y: f32) -> Point {
    Point::new(x / CELL, (FONT_TOP_Y - y) / CELL)
}

/// Flatten stored outline paths into closed polygons in grid coordinates.
fn flatten_path(path: &str) -> Vec<Vec<Point>> {
    let Some(tokens) = tokenize(path) else {
        return Vec::new();
    };
    let mut polygons: Vec<Vec<Point>> = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    let mut cursor = Point::new(0.0, 0.0);
    let mut subpath_start = cursor;

    // Each arm reads its numbers; a missing one abandons the whole path.
    let arm = |tokens: &[Token], i: &mut usize| -> Option<f32> {
        *i += 1;
        match tokens.get(*i) {
            Some(Token::Number(n)) => Some(*n),
            _ => None,
        }
    };
    let point = |tokens: &[Token], i: &mut usize| -> Option<Point> {
        let x = arm(tokens, i)?;
        let y = arm(tokens, i)?;
        Some(Point::new(x, y))
    };

    let mut i = 0usize;
    while i < tokens.len() {
        let Token::Command(cmd) = tokens[i] else {
            // A number where a command belongs is an implicit repeat, which this
            // data never contains.
            return Vec::new();
        };
        let result = match cmd {
            b'M' => point(&tokens, &mut i).map(|p| {
                if current.len() >= 3 {
                    polygons.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
                let p = to_grid(p.x, p.y);
                current.push(p);
                cursor = p;
                subpath_start = p;
            }),
            b'L' => point(&tokens, &mut i).map(|p| {
                let p = to_grid(p.x, p.y);
                current.push(p);
                cursor = p;
            }),
            b'Q' => {
                let control = point(&tokens, &mut i);
                let end = point(&tokens, &mut i);
                match (control, end) {
                    (Some(c), Some(e)) => {
                        let c = to_grid(c.x, c.y);
                        let e = to_grid(e.x, e.y);
                        flatten_quadratic(cursor, c, e, &mut current);
                        cursor = e;
                        Some(())
                    }
                    _ => None,
                }
            }
            b'C' => {
                let c1 = point(&tokens, &mut i);
                let c2 = point(&tokens, &mut i);
                let end = point(&tokens, &mut i);
                match (c1, c2, end) {
                    (Some(a), Some(b), Some(e)) => {
                        let a = to_grid(a.x, a.y);
                        let b = to_grid(b.x, b.y);
                        let e = to_grid(e.x, e.y);
                        flatten_cubic(cursor, a, b, e, &mut current);
                        cursor = e;
                        Some(())
                    }
                    _ => None,
                }
            }
            b'Z' | b'z' => {
                if current.len() >= 3 {
                    polygons.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
                cursor = subpath_start;
                Some(())
            }
            _ => None,
        };
        if result.is_none() {
            return Vec::new();
        }
        i += 1;
    }
    if current.len() >= 3 {
        polygons.push(current);
    }
    polygons
}

/// Subdivisions for a curve, from the length of its control polygon.
fn steps(control_length: f32) -> usize {
    ((control_length / FLATTEN_CHORD).ceil() as usize).clamp(4, 96)
}

fn flatten_quadratic(from: Point, control: Point, to: Point, out: &mut Vec<Point>) {
    let length = from.distance_to(control) + control.distance_to(to);
    let n = steps(length);
    for k in 1..=n {
        let t = k as f32 / n as f32;
        let u = 1.0 - t;
        out.push(Point::new(
            u * u * from.x + 2.0 * u * t * control.x + t * t * to.x,
            u * u * from.y + 2.0 * u * t * control.y + t * t * to.y,
        ));
    }
}

fn flatten_cubic(from: Point, c1: Point, c2: Point, to: Point, out: &mut Vec<Point>) {
    let length = from.distance_to(c1) + c1.distance_to(c2) + c2.distance_to(to);
    let n = steps(length);
    for k in 1..=n {
        let t = k as f32 / n as f32;
        let u = 1.0 - t;
        out.push(Point::new(
            u * u * u * from.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * to.x,
            u * u * u * from.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * to.y,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(a: (f32, f32), b: (f32, f32), n: usize) -> Vec<Point> {
        (0..n)
            .map(|i| {
                let t = i as f32 / (n - 1) as f32;
                Point::new(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
            })
            .collect()
    }

    /// A filled rectangle in font space, as one closed path.
    fn rect_font(x0: f32, y0: f32, x1: f32, y1: f32) -> String {
        format!("M {x0} {y0} L {x1} {y0} L {x1} {y1} L {x0} {y1} Z")
    }

    /// Inked cells in a mask. The tests live inside the module, so they can read
    /// the count directly rather than through an accessor only they would use.
    fn cells(ink: &Ink) -> u32 {
        ink.count
    }

    /// Intersection over union, which the production measure deliberately does
    /// not use — see the module docs. Kept here because these tests are about
    /// what the rasteriser paints.
    fn iou(a: &Ink, b: &Ink) -> f32 {
        let inter = a.intersection_count(b);
        let union = a.count + b.count - inter;
        if union == 0 {
            return 1.0;
        }
        inter as f32 / union as f32
    }

    #[test]
    fn a_filled_rectangle_has_the_area_you_asked_for() {
        // 100x200 font-space units, so 2500 cells at 4 units per cell.
        let ink = Ink::from_outlines(&[&rect_font(100.0, 100.0, 200.0, 300.0)]);
        assert!(!ink.is_empty());
        let expected = (100.0 / CELL) * (200.0 / CELL);
        let error = (cells(&ink) as f32 - expected).abs() / expected;
        assert!(error < 0.05, "{} cells against {expected}", cells(&ink));
    }

    /// The vertical centre of a mask's ink, in grid rows.
    fn centre_row(ink: &Ink) -> f32 {
        let (mut sum, mut n) = (0.0f32, 0.0f32);
        for py in 0..GRID {
            for px in 0..GRID {
                if ink.bits[(py * GRID + px) / 64] & (1 << (px % 64)) != 0 {
                    sum += py as f32;
                    n += 1.0;
                }
            }
        }
        assert!(n > 0.0, "mask is empty");
        sum / n
    }

    #[test]
    fn font_and_display_space_agree_with_the_canvas() {
        // A rectangle at the top of the font box must land at the top of the
        // display grid, because the outline is y-up and the grid is y-down.
        let top = Ink::from_outlines(&[&rect_font(100.0, 700.0, 900.0, 880.0)]);
        let bottom = Ink::from_outlines(&[&rect_font(100.0, 20.0, 900.0, 200.0)]);
        assert!(
            centre_row(&top) < centre_row(&bottom),
            "the font box's top should rasterise above its bottom: {} vs {}",
            centre_row(&top),
            centre_row(&bottom)
        );
    }

    #[test]
    fn a_straight_stroke_is_as_thick_as_its_pen() {
        let stroke = vec![line((100.0, 500.0), (900.0, 500.0), 4)];
        let ink = Ink::from_strokes(&stroke, 40.0);
        // 800 long, 40 wide, plus two round caps of radius 20.
        let expected = (800.0 * 40.0 + std::f32::consts::PI * 20.0 * 20.0) / (CELL * CELL);
        let error = (cells(&ink) as f32 - expected).abs() / expected;
        assert!(error < 0.08, "{} cells against {expected}", cells(&ink));
    }

    #[test]
    fn a_thinner_pen_covers_less_paper() {
        let stroke = vec![line((100.0, 500.0), (900.0, 500.0), 4)];
        let fat = Ink::from_strokes(&stroke, 40.0);
        let thin = Ink::from_strokes(&stroke, 40.0 / 3.0);
        assert!(cells(&thin) < cells(&fat) / 2, "{} vs {}", cells(&thin), cells(&fat));
        assert!(iou(&thin, &fat) < 0.45, "thin/fat IoU {}", iou(&thin, &fat));
    }

    #[test]
    fn overshooting_adds_ink_without_adding_coverage() {
        let reference = Ink::from_strokes(&[line((300.0, 500.0), (700.0, 500.0), 4)], 40.0);
        let straight = Ink::from_strokes(&[line((300.0, 500.0), (700.0, 500.0), 4)], 40.0);
        let overshoot = Ink::from_strokes(&[line((100.0, 500.0), (950.0, 500.0), 4)], 40.0);
        assert_eq!(iou(&straight, &reference), 1.0);
        assert!(iou(&overshoot, &reference) < 0.7, "{}", iou(&overshoot, &reference));
        // The reference is still entirely covered; it is the extra ink that hurts.
        assert_eq!(overshoot.intersection_count(&reference), cells(&reference));
    }

    #[test]
    fn a_missing_outline_is_not_a_fault() {
        assert!(Ink::from_outlines(&[]).is_empty());
        assert!(Ink::from_outlines(&[""]).is_empty());
        assert!(Ink::from_outlines(&["garbage here"]).is_empty());
        // An unsupported relative command abandons the path rather than
        // inventing geometry for it.
        assert!(Ink::from_outlines(&["m 10 10 l 20 20 z"]).is_empty());
    }

    #[test]
    fn curves_are_flattened_into_a_closed_shape() {
        // A quadratic "leaf": a curve out and the same curve back. A quadratic
        // bulging `h` above a chord of length `b` encloses `2/3 * b * h` with the
        // chord, so the lens is exactly twice that.
        let path = "M 200 500 Q 512 700 824 500 Q 512 300 200 500 Z";
        let ink = Ink::from_outlines(&[path]);
        assert!(!ink.is_empty());
        let expected = 2.0 * (2.0 / 3.0) * 624.0 * 100.0 / (CELL * CELL);
        let error = (cells(&ink) as f32 - expected).abs() / expected;
        assert!(error < 0.05, "{} cells against about {expected}", cells(&ink));
    }

    #[test]
    fn empty_masks_agree_with_themselves() {
        let a = Ink::empty();
        let b = Ink::empty();
        assert_eq!(iou(&a, &b), 1.0);
        assert!(a.is_empty());
        let some = Ink::from_strokes(&[line((0.0, 0.0), (100.0, 0.0), 2)], 20.0);
        assert_eq!(iou(&a, &some), 0.0);
    }

    #[test]
    fn masks_union_without_double_counting() {
        let a = Ink::from_strokes(&[line((200.0, 200.0), (800.0, 200.0), 2)], 40.0);
        let b = Ink::from_strokes(&[line((200.0, 200.0), (800.0, 200.0), 2)], 40.0);
        let mut union = a.clone();
        union.or_with(&b);
        assert_eq!(cells(&union), cells(&a), "identical masks must not double");
        assert_eq!(a.intersection_count(&b), cells(&a));
    }
}
