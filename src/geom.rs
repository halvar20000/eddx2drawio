//! 2D geometry: affine transforms and path segments (y axis points down).

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

pub fn pt(x: f64, y: f64) -> Pt {
    Pt { x, y }
}

/// x' = a x + c y + e ; y' = b x + d y + f
#[derive(Clone, Copy, Debug)]
pub struct Affine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Affine {
    pub const IDENTITY: Affine = Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    pub fn translate(x: f64, y: f64) -> Affine {
        Affine { e: x, f: y, ..Affine::IDENTITY }
    }

    /// Rotation by `deg` degrees; positive turns clockwise on screen (y down).
    pub fn rotate(deg: f64) -> Affine {
        let (s, c) = deg.to_radians().sin_cos();
        Affine { a: c, b: s, c: -s, d: c, e: 0.0, f: 0.0 }
    }

    pub fn scale(sx: f64, sy: f64) -> Affine {
        Affine { a: sx, d: sy, ..Affine::IDENTITY }
    }

    /// `self ∘ other`: apply `other` first, then `self`.
    pub fn then(&self, other: &Affine) -> Affine {
        let m = self;
        let n = other;
        Affine {
            a: m.a * n.a + m.c * n.b,
            b: m.b * n.a + m.d * n.b,
            c: m.a * n.c + m.c * n.d,
            d: m.b * n.c + m.d * n.d,
            e: m.a * n.e + m.c * n.f + m.e,
            f: m.b * n.e + m.d * n.f + m.f,
        }
    }

    pub fn apply(&self, p: Pt) -> Pt {
        pt(self.a * p.x + self.c * p.y + self.e, self.b * p.x + self.d * p.y + self.f)
    }

    pub fn det(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    pub fn invert(&self) -> Affine {
        let det = self.det();
        if det.abs() < 1e-12 {
            return Affine::IDENTITY;
        }
        let a = self.d / det;
        let b = -self.b / det;
        let c = -self.c / det;
        let d = self.a / det;
        Affine { a, b, c, d, e: -(a * self.e + c * self.f), f: -(b * self.e + d * self.f) }
    }

    /// Split a rigid (rotation + optional mirror) linear part into a clockwise
    /// rotation in degrees and a horizontal flip, as draw.io stores them.
    pub fn rotation_and_flip(&self) -> (f64, bool) {
        let flip = self.det() < 0.0;
        // with a horizontal flip the first column is -(cos, sin)
        let (cos, sin) = if flip { (-self.a, -self.b) } else { (self.a, self.b) };
        let mut deg = sin.atan2(cos).to_degrees();
        if deg.abs() < 1e-6 {
            deg = 0.0;
        }
        if deg < 0.0 {
            deg += 360.0;
        }
        (deg, flip)
    }
}

#[derive(Clone, Debug)]
pub enum Seg {
    Move(Pt),
    Line(Pt),
    /// cubic Bézier: control 1, control 2, end
    Cubic(Pt, Pt, Pt),
    Close,
}

impl Seg {
    pub fn map(&self, m: &Affine) -> Seg {
        match self {
            Seg::Move(p) => Seg::Move(m.apply(*p)),
            Seg::Line(p) => Seg::Line(m.apply(*p)),
            Seg::Cubic(a, b, p) => Seg::Cubic(m.apply(*a), m.apply(*b), m.apply(*p)),
            Seg::Close => Seg::Close,
        }
    }

    pub fn end(&self) -> Option<Pt> {
        match self {
            Seg::Move(p) | Seg::Line(p) | Seg::Cubic(_, _, p) => Some(*p),
            Seg::Close => None,
        }
    }
}

/// Circle through three points: (centre, radius), or None if they are (nearly) collinear.
fn circle_through(p0: Pt, p1: Pt, p2: Pt) -> Option<(Pt, f64)> {
    let d = 2.0 * (p0.x * (p1.y - p2.y) + p1.x * (p2.y - p0.y) + p2.x * (p0.y - p1.y));
    if d.abs() < 1e-9 {
        return None;
    }
    let s0 = p0.x * p0.x + p0.y * p0.y;
    let s1 = p1.x * p1.x + p1.y * p1.y;
    let s2 = p2.x * p2.x + p2.y * p2.y;
    let cx = (s0 * (p1.y - p2.y) + s1 * (p2.y - p0.y) + s2 * (p0.y - p1.y)) / d;
    let cy = (s0 * (p2.x - p1.x) + s1 * (p0.x - p2.x) + s2 * (p1.x - p0.x)) / d;
    let r = ((p0.x - cx).powi(2) + (p0.y - cy).powi(2)).sqrt();
    Some((pt(cx, cy), r))
}

/// Circular arc from `start` through `mid` to `end`, as cubic segments.
pub fn arc_through(start: Pt, mid: Pt, end: Pt) -> Vec<Seg> {
    let Some((c, r)) = circle_through(start, mid, end) else {
        return vec![Seg::Line(end)];
    };
    let ang = |p: Pt| (p.y - c.y).atan2(p.x - c.x);
    let a0 = ang(start);
    let am = ang(mid);
    let a1 = ang(end);
    let norm = |a: f64| {
        let t = std::f64::consts::TAU;
        ((a % t) + t) % t
    };
    // sweep from a0 towards a1 in the direction that passes through mid
    let ccw_to_end = norm(a1 - a0);
    let ccw_to_mid = norm(am - a0);
    let sweep = if ccw_to_mid <= ccw_to_end { ccw_to_end } else { ccw_to_end - std::f64::consts::TAU };
    let n = (sweep.abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = sweep / n as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let mut out = Vec::with_capacity(n);
    let mut a = a0;
    for i in 0..n {
        let b = a + step;
        let (sa, ca) = a.sin_cos();
        let (sb, cb) = b.sin_cos();
        let p0 = pt(c.x + r * ca, c.y + r * sa);
        let p3 = if i == n - 1 { end } else { pt(c.x + r * cb, c.y + r * sb) };
        let c1 = pt(p0.x - k * r * sa, p0.y + k * r * ca);
        let c2 = pt(p3.x + k * r * sb, p3.y - k * r * cb);
        out.push(Seg::Cubic(c1, c2, p3));
        a = b;
    }
    out
}

/// Visio-style ArcTo: arc from `start` to `end` whose midpoint lies `bow` away
/// from the chord's midpoint.
pub fn arc_bow(start: Pt, end: Pt, bow: f64) -> Vec<Seg> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len = (dx * dx + dy * dy).sqrt();
    if bow.abs() < 1e-9 || len < 1e-9 {
        return vec![Seg::Line(end)];
    }
    let m = pt((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    // normal pointing to the left of the chord in y-up terms (= right on screen)
    let n = pt(dy / len, -dx / len);
    let mid = pt(m.x + n.x * bow * ARC_BOW_SIGN, m.y + n.y * bow * ARC_BOW_SIGN);
    arc_through(start, mid, end)
}

/// Direction of a positive ArcTo bow (checked against EdrawMax's own previews).
pub const ARC_BOW_SIGN: f64 = 1.0;

/// Visio-style EllipseArcTo: elliptical arc from `start` through `ctrl` to `end`;
/// `angle` is the major axis angle (radians), `ratio` = major / minor axis.
pub fn ellipse_arc(start: Pt, ctrl: Pt, end: Pt, angle: f64, ratio: f64) -> Vec<Seg> {
    if !(ratio.is_finite()) || ratio.abs() < 1e-9 {
        return vec![Seg::Line(end)];
    }
    // map the ellipse onto a circle, solve there, map back
    let to_circle = Affine::scale(1.0 / ratio, 1.0).then(&Affine::rotate(-angle.to_degrees()));
    let back = to_circle.invert();
    arc_through(to_circle.apply(start), to_circle.apply(ctrl), to_circle.apply(end))
        .into_iter()
        .map(|s| s.map(&back))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Pt, b: Pt) -> bool {
        (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6
    }

    #[test]
    fn half_circle_through_top_point() {
        let segs = arc_through(pt(-1.0, 0.0), pt(0.0, -1.0), pt(1.0, 0.0));
        assert_eq!(segs.len(), 2);
        let Seg::Cubic(_, _, p) = segs[0] else { panic!() };
        assert!(close(p, pt(0.0, -1.0)));
    }

    #[test]
    fn decompose_rotation_and_flip() {
        let m = Affine::rotate(90.0);
        let (deg, flip) = m.rotation_and_flip();
        assert!((deg - 90.0).abs() < 1e-9 && !flip);
        let f = Affine::rotate(30.0).then(&Affine::scale(-1.0, 1.0));
        let (deg, flip) = f.rotation_and_flip();
        assert!((deg - 30.0).abs() < 1e-9 && flip);
    }

    #[test]
    fn ellipse_arc_hits_control_point() {
        let segs = ellipse_arc(pt(0.0, 0.0), pt(2.0, 1.0), pt(4.0, 0.0), 0.0, 2.0);
        let ends: Vec<Pt> = segs.iter().filter_map(|s| s.end()).collect();
        assert!(close(*ends.last().unwrap(), pt(4.0, 0.0)));
    }

    #[test]
    fn invert_roundtrip() {
        let m = Affine::translate(3.0, 4.0).then(&Affine::rotate(33.0)).then(&Affine::scale(2.0, 0.5));
        let p = pt(1.5, -2.0);
        assert!(close(m.invert().apply(m.apply(p)), p));
    }
}
