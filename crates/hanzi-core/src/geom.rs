//! Stroke geometry: resampling, normalisation and distance measures.
//!
//! Two coordinate systems appear throughout this crate:
//!
//! * **font space** — exactly as published by Make Me a Hanzi. The upper-left of
//!   the character box is `(0, 900)` and the lower-right is `(1024, -124)`, so
//!   the y axis *increases upwards*. Stroke outlines are kept in this space
//!   because they are rendered verbatim as SVG/canvas path data.
//! * **display space** — `x` and `y` both run `0..=1024` with the y axis
//!   pointing *down*, matching the canvas the user draws on. All grading
//!   happens here.
//!
//! [`Point::from_font`] converts between the two.

use serde::{Deserialize, Serialize};

/// Width and height of the character design box, in design units.
pub const EM: f32 = 1024.0;

/// The font-space y coordinate of the top edge of the character box.
///
/// Make Me a Hanzi documents the upper-left corner as `(0, 900)`, so the
/// vertical span of the box is `-124..=900` and y grows upwards.
pub const FONT_TOP_Y: f32 = 900.0;

/// A point on a stroke, in display space unless stated otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Build a display-space point from Make Me a Hanzi font coordinates,
    /// flipping the y axis so that y grows downwards.
    pub fn from_font(x: f32, y: f32) -> Self {
        Self::new(x, FONT_TOP_Y - y)
    }

    pub fn distance_to(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Total length of a polyline.
pub fn path_length(points: &[Point]) -> f32 {
    points
        .windows(2)
        .map(|w| w[0].distance_to(w[1]))
        .sum()
}

/// Total length of every stroke in an attempt.
pub fn strokes_length(strokes: &[Vec<Point>]) -> f32 {
    strokes.iter().map(|s| path_length(s)).sum()
}

/// Resample a polyline to exactly `k` points spaced evenly along its arc
/// length, preserving the original start and end points.
///
/// Degenerate input (empty, single point, or zero length) is handled by
/// repeating whatever point is available, so the result always has `k` points
/// when `k >= 2`, which keeps downstream index-wise comparisons well defined.
pub fn resample(points: &[Point], k: usize) -> Vec<Point> {
    let k = k.max(2);
    if points.is_empty() {
        return Vec::new();
    }
    if points.len() == 1 {
        return vec![points[0]; k];
    }
    let total = path_length(points);
    if total <= f32::EPSILON {
        return vec![points[0]; k];
    }

    let step = total / (k - 1) as f32;
    let mut out = Vec::with_capacity(k);
    out.push(points[0]);

    let mut seg = 0usize;
    let mut consumed = 0.0f32;
    let mut target = step;

    while out.len() < k - 1 {
        // Walk forward until `target` falls inside the current segment.
        loop {
            if seg + 1 >= points.len() {
                break;
            }
            let seg_len = points[seg].distance_to(points[seg + 1]);
            if consumed + seg_len >= target {
                break;
            }
            consumed += seg_len;
            seg += 1;
        }
        if seg + 1 >= points.len() {
            break;
        }
        let seg_len = points[seg].distance_to(points[seg + 1]);
        let t = if seg_len <= f32::EPSILON {
            0.0
        } else {
            (target - consumed) / seg_len
        };
        let a = points[seg];
        let b = points[seg + 1];
        out.push(Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        target += step;
    }

    // Floating-point drift can leave us a point or two short.
    while out.len() < k {
        out.push(*points.last().expect("non-empty"));
    }
    out
}

/// Mean position of a set of points. Returns the origin for empty input.
pub fn centroid(points: &[Point]) -> Point {
    if points.is_empty() {
        return Point::new(0.0, 0.0);
    }
    let n = points.len() as f32;
    let sx: f32 = points.iter().map(|p| p.x).sum();
    let sy: f32 = points.iter().map(|p| p.y).sum();
    Point::new(sx / n, sy / n)
}

/// Root-mean-square distance of a point set from its own centroid.
///
/// Used as a cheap, rotation-free measure of "how big is this shape", which is
/// enough to normalise scale before comparing stroke shapes.
pub fn rms_radius(points: &[Point]) -> f32 {
    if points.is_empty() {
        return 0.0;
    }
    let c = centroid(points);
    let sum: f32 = points.iter().map(|p| p.distance_to(c).powi(2)).sum();
    (sum / points.len() as f32).sqrt()
}

/// Axis-aligned bounding box as `(min, max)`. Returns `None` for empty input.
pub fn bbox(points: &[Point]) -> Option<(Point, Point)> {
    let mut it = points.iter();
    let first = *it.next()?;
    let mut min = first;
    let mut max = first;
    for p in it {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    Some((min, max))
}

/// Translate to the origin and scale to unit RMS radius.
///
/// Returns `None` when the shape is essentially a single point, in which case
/// it carries no scale-free shape information at all.
pub fn normalize(points: &[Point]) -> Option<Vec<Point>> {
    let radius = rms_radius(points);
    if radius <= 1e-4 {
        return None;
    }
    let c = centroid(points);
    Some(
        points
            .iter()
            .map(|p| Point::new((p.x - c.x) / radius, (p.y - c.y) / radius))
            .collect(),
    )
}

/// Mean distance between two equally long polylines, compared index by index.
fn mean_index_distance(a: &[Point], b: &[Point]) -> f32 {
    let n = a.len().min(b.len());
    if n == 0 {
        return f32::INFINITY;
    }
    let sum: f32 = (0..n).map(|i| a[i].distance_to(b[i])).sum();
    sum / n as f32
}

/// Scale- and translation-invariant distance between two stroke shapes,
/// measured both forwards and with the second stroke reversed.
///
/// `forward <= reversed` means the attempt was drawn in the same direction as
/// the reference; the reverse indicates the stroke was drawn backwards (for
/// example a 横 written right-to-left).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeDistance {
    pub forward: f32,
    pub reversed: f32,
}

impl ShapeDistance {
    /// True when the attempt clearly runs opposite to the reference.
    ///
    /// The margin guards against short, near-symmetric strokes (notably 点),
    /// where forwards and reversed distances are naturally almost equal and
    /// direction is simply not observable.
    pub fn is_reversed(self) -> bool {
        self.reversed < self.forward * 0.85
    }

    pub fn best(self) -> f32 {
        self.forward.min(self.reversed)
    }
}

/// Compare two polylines for shape alone, ignoring where they sit and how big
/// they are. `k` controls how many points each is resampled to.
pub fn shape_distance(a: &[Point], b: &[Point], k: usize) -> ShapeDistance {
    let ra = resample(a, k);
    let rb = resample(b, k);
    let (Some(na), Some(nb)) = (normalize(&ra), normalize(&rb)) else {
        // At least one side is a dot: there is no shape to disagree about.
        return ShapeDistance {
            forward: 0.0,
            reversed: 0.0,
        };
    };
    let mut rev = nb.clone();
    rev.reverse();
    ShapeDistance {
        forward: mean_index_distance(&na, &nb),
        reversed: mean_index_distance(&na, &rev),
    }
}

/// A uniform scale plus translation that maps one attempt onto another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    pub scale: f32,
    pub from_centroid: Point,
    pub to_centroid: Point,
}

impl Fit {
    pub fn apply(&self, p: Point) -> Point {
        Point::new(
            (p.x - self.from_centroid.x) * self.scale + self.to_centroid.x,
            (p.y - self.from_centroid.y) * self.scale + self.to_centroid.y,
        )
    }
}

/// Best-fit uniform scale and translation aligning `from` onto `to`.
///
/// The scale is the ratio of RMS radii and is clamped so that a wild attempt
/// cannot be "rescued" by an absurd zoom.
pub fn fit_similarity(from: &[Point], to: &[Point]) -> Fit {
    let from_radius = rms_radius(from);
    let to_radius = rms_radius(to);
    let scale = if from_radius > 1e-3 {
        (to_radius / from_radius).clamp(0.5, 2.0)
    } else {
        1.0
    };
    Fit {
        scale,
        from_centroid: centroid(from),
        to_centroid: centroid(to),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn font_space_flips_y() {
        assert_eq!(Point::from_font(0.0, 900.0), Point::new(0.0, 0.0));
        assert_eq!(Point::from_font(1024.0, -124.0), Point::new(1024.0, 1024.0));
    }

    #[test]
    fn resample_spaces_points_evenly() {
        let line = vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0)];
        let out = resample(&line, 5);
        assert_eq!(out.len(), 5);
        for (i, p) in out.iter().enumerate() {
            assert!(approx(p.x, i as f32 * 25.0, 1e-3), "point {i} at {p:?}");
        }
    }

    #[test]
    fn resample_handles_degenerate_input() {
        assert!(resample(&[], 8).is_empty());
        let dot = resample(&[Point::new(3.0, 4.0)], 8);
        assert_eq!(dot.len(), 8);
        assert!(dot.iter().all(|p| *p == Point::new(3.0, 4.0)));
        let dup = resample(&[Point::new(1.0, 1.0), Point::new(1.0, 1.0)], 4);
        assert_eq!(dup.len(), 4);
    }

    #[test]
    fn resample_walks_a_corner() {
        let corner = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ];
        let out = resample(&corner, 3);
        assert!(approx(out[0].x, 0.0, 1e-4) && approx(out[0].y, 0.0, 1e-4));
        assert!(approx(out[1].x, 10.0, 1e-4) && approx(out[1].y, 0.0, 1e-4));
        assert!(approx(out[2].x, 10.0, 1e-4) && approx(out[2].y, 10.0, 1e-4));
    }

    #[test]
    fn identity_shape_has_zero_distance() {
        let s = vec![
            Point::new(0.0, 0.0),
            Point::new(50.0, 10.0),
            Point::new(100.0, 0.0),
        ];
        let d = shape_distance(&s, &s, 16);
        assert!(approx(d.forward, 0.0, 1e-4));
        assert!(!d.is_reversed());
    }

    #[test]
    fn shape_distance_ignores_position_and_scale() {
        let a = vec![
            Point::new(0.0, 0.0),
            Point::new(50.0, 10.0),
            Point::new(100.0, 0.0),
        ];
        let b: Vec<Point> = a
            .iter()
            .map(|p| Point::new(p.x * 3.0 + 200.0, p.y * 3.0 - 400.0))
            .collect();
        let d = shape_distance(&a, &b, 16);
        assert!(d.forward < 0.05, "expected near zero, got {d:?}");
    }

    #[test]
    fn reversed_stroke_is_detected() {
        let forward = vec![
            Point::new(0.0, 0.0),
            Point::new(60.0, 0.0),
            Point::new(120.0, 12.0),
        ];
        let backward: Vec<Point> = forward.iter().rev().copied().collect();
        let d = shape_distance(&forward, &backward, 16);
        assert!(d.is_reversed(), "expected reversal, got {d:?}");
        assert!(d.reversed < d.forward);
    }

    #[test]
    fn a_horizontal_and_a_vertical_are_far_apart() {
        let horizontal = vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0)];
        let vertical = vec![Point::new(0.0, 0.0), Point::new(0.0, 100.0)];
        let d = shape_distance(&horizontal, &vertical, 16);
        assert!(d.best() > 0.8, "expected a large distance, got {d:?}");
    }

    #[test]
    fn fit_maps_centroid_and_radius() {
        let from = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(0.0, 10.0),
        ];
        let to: Vec<Point> = from
            .iter()
            .map(|p| Point::new(p.x * 2.0 + 100.0, p.y * 2.0 + 50.0))
            .collect();
        let fit = fit_similarity(&from, &to);
        assert!(approx(fit.scale, 2.0, 1e-3), "scale {}", fit.scale);
        for (a, b) in from.iter().zip(to.iter()) {
            let mapped = fit.apply(*a);
            assert!(mapped.distance_to(*b) < 1e-3, "{mapped:?} vs {b:?}");
        }
    }
}
