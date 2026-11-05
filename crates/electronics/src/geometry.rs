use crate::{Error, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub type Point = [f64; 2];
const EPS: f64 = 1e-9;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Polygon {
    /// Implicitly closed rings; no repeated final vertex. Simple polygons only.
    pub outer: Vec<Point>,
    #[serde(default)]
    pub holes: Vec<Vec<Point>>,
}

pub(crate) fn number(v: f64, min: f64, max: f64, what: &str) -> Result<()> {
    if !v.is_finite() || v < min || v > max {
        return Err(Error(format!("{what} must be finite in {min}..={max}")));
    }
    Ok(())
}
pub(crate) fn point(p: Point) -> Result<()> {
    for v in p {
        number(v, -1e6, 1e6, "coordinate (mm)")?;
    }
    Ok(())
}
pub(crate) fn path(points: &[Point]) -> Result<()> {
    if !(2..=4096).contains(&points.len()) {
        return Err(Error("path needs 2..4096 points".into()));
    }
    for &p in points {
        point(p)?;
    }
    if points.windows(2).any(|w| w.first() == w.get(1)) {
        return Err(Error("zero-length path segment".into()));
    }
    Ok(())
}
fn cross(a: Point, b: Point, c: Point) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn on_segment(a: Point, b: Point, p: Point) -> bool {
    cross(a, b, p).abs() <= EPS && (0..2).all(|i| p[i] >= a[i].min(b[i]) - EPS && p[i] <= a[i].max(b[i]) + EPS)
}
fn intersects(a: Point, b: Point, c: Point, d: Point) -> bool {
    let (x, y, z, w) = (cross(a, b, c), cross(a, b, d), cross(c, d, a), cross(c, d, b));
    ((x > EPS && y < -EPS || x < -EPS && y > EPS) && (z > EPS && w < -EPS || z < -EPS && w > EPS))
        || on_segment(a, b, c)
        || on_segment(a, b, d)
        || on_segment(c, d, a)
        || on_segment(c, d, b)
}
fn edges(r: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    r.iter().copied().zip(r.iter().copied().cycle().skip(1)).take(r.len())
}
fn ring(r: &[Point]) -> Result<()> {
    if !(3..=512).contains(&r.len()) {
        return Err(Error("polygon ring needs 3..512 vertices".into()));
    }
    for &p in r {
        point(p)?;
    }
    let area = edges(r).map(|(a, b)| a[0] * b[1] - b[0] * a[1]).sum::<f64>();
    if area.abs() <= EPS {
        return Err(Error("polygon has zero area".into()));
    }
    for (i, (a, b)) in edges(r).enumerate() {
        if a == b {
            return Err(Error("polygon has a zero-length edge".into()));
        }
        for (j, (c, d)) in edges(r).enumerate().skip(i + 1) {
            if j == i + 1 || i == 0 && j == r.len() - 1 {
                continue;
            }
            if intersects(a, b, c, d) {
                return Err(Error("polygon self-intersects".into()));
            }
        }
        // Adjacent collinear edges must not backtrack over one another.
        let c = r[(i + 2) % r.len()];
        if cross(a, b, c).abs() <= EPS && ((a[0] - b[0]) * (c[0] - b[0]) + (a[1] - b[1]) * (c[1] - b[1])) > EPS {
            return Err(Error("polygon edges overlap".into()));
        }
    }
    Ok(())
}
fn inside(r: &[Point], p: Point) -> bool {
    if edges(r).any(|(a, b)| on_segment(a, b, p)) {
        return true;
    }
    edges(r).filter(|(a, b)| (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]).count() % 2 == 1
}
impl Polygon {
    pub fn validate(&self) -> Result<()> {
        ring(&self.outer)?;
        if self.holes.len() > 64 || self.outer.len() + self.holes.iter().map(Vec::len).sum::<usize>() > 1024 {
            return Err(Error("polygon exceeds vertex/hole budget".into()));
        }
        for (i, h) in self.holes.iter().enumerate() {
            ring(h)?;
            if !h.iter().all(|&p| inside(&self.outer, p)) || edges(h).any(|(a, b)| edges(&self.outer).any(|(c, d)| intersects(a, b, c, d))) {
                return Err(Error("hole touches or leaves outer boundary".into()));
            }
            for prior in self.holes.iter().take(i) {
                if h.iter().any(|&p| inside(prior, p))
                    || prior.iter().any(|&p| inside(h, p))
                    || edges(h).any(|(a, b)| edges(prior).any(|(c, d)| intersects(a, b, c, d)))
                {
                    return Err(Error("holes overlap or nest".into()));
                }
            }
        }
        Ok(())
    }
    pub fn contains(&self, p: Point) -> bool {
        inside(&self.outer, p) && !self.holes.iter().any(|h| inside(h, p))
    }
}
