use macroquad::prelude::*;

pub fn rnd(a: f32, b: f32) -> f32 {
    macroquad::rand::gen_range(a, b)
}

pub fn dir(a: f32) -> Vec2 {
    vec2(a.cos(), a.sin())
}

pub fn angle_of(v: Vec2) -> f32 {
    v.y.atan2(v.x)
}

/// Shortest signed difference a - b, wrapped to -PI..PI.
pub fn angle_diff(a: f32, b: f32) -> f32 {
    let mut d = (a - b) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    d
}

/// Does the segment a->b touch the rectangle? (slab method)
pub fn seg_hits_rect(a: Vec2, b: Vec2, r: &Rect) -> bool {
    let d = b - a;
    let mut t0 = 0.0f32;
    let mut t1 = 1.0f32;
    let checks = [
        (-d.x, a.x - r.x),
        (d.x, r.x + r.w - a.x),
        (-d.y, a.y - r.y),
        (d.y, r.y + r.h - a.y),
    ];
    for (p, q) in checks {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                if t > t1 {
                    return false;
                }
                if t > t0 {
                    t0 = t;
                }
            } else {
                if t < t0 {
                    return false;
                }
                if t < t1 {
                    t1 = t;
                }
            }
        }
    }
    true
}

/// Closest point on the segment a->b to point p.
pub fn closest_on_seg(a: Vec2, b: Vec2, p: Vec2) -> Vec2 {
    let ab = b - a;
    let l2 = ab.length_squared();
    if l2 < 1e-6 {
        return a;
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    a + ab * t
}

/// Push vector that moves a circle out of a rectangle, if overlapping.
pub fn circle_rect_push(c: Vec2, r: f32, rect: &Rect) -> Option<Vec2> {
    let cx = c.x.clamp(rect.x, rect.x + rect.w);
    let cy = c.y.clamp(rect.y, rect.y + rect.h);
    let d = c - vec2(cx, cy);
    let l2 = d.length_squared();
    if l2 >= r * r {
        return None;
    }
    if l2 > 1e-6 {
        let l = l2.sqrt();
        Some(d / l * (r - l))
    } else {
        Some(vec2(0.0, -r))
    }
}

pub fn dist_point_rect(p: Vec2, rect: &Rect) -> f32 {
    let cx = p.x.clamp(rect.x, rect.x + rect.w);
    let cy = p.y.clamp(rect.y, rect.y + rect.h);
    p.distance(vec2(cx, cy))
}
