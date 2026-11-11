use crate::util::*;
use crate::weapons::WKind;
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub const MAP: f32 = 3200.0;
pub const CELL: f32 = 40.0;
pub const GN: usize = 80;

const PHASES: usize = 5;
const R_END: [f32; PHASES] = [1400.0, 850.0, 480.0, 240.0, 90.0];
const WAIT: [f32; PHASES] = [45.0, 35.0, 30.0, 25.0, 20.0];
const SHRINK: [f32; PHASES] = [30.0, 25.0, 20.0, 15.0, 12.0];
const DMG: [f32; PHASES] = [1.0, 2.0, 4.0, 7.0, 12.0];

#[derive(Clone, Copy, PartialEq)]
pub enum LootKind {
    Weapon(WKind),
    Armor,
    Medkit,
}

pub struct Loot {
    pub pos: Vec2,
    pub kind: LootKind,
    pub taken: bool,
}

pub struct Zone {
    pub c: Vec2,
    pub r: f32,
    pub c0: Vec2,
    pub r0: f32,
    pub c1: Vec2,
    pub r1: f32,
    pub phase: usize,
    pub timer: f32,
    pub shrinking: bool,
    pub done: bool,
}

impl Zone {
    pub fn new() -> Self {
        let c = vec2(MAP / 2.0, MAP / 2.0);
        let mut z = Zone {
            c, r: 2300.0, c0: c, r0: 2300.0, c1: c, r1: R_END[0],
            phase: 0, timer: 0.0, shrinking: false, done: false,
        };
        z.pick_next();
        z
    }

    fn pick_next(&mut self) {
        let rn = R_END[self.phase];
        let maxoff = (self.r0 - rn).max(0.0) * 0.8;
        let off = dir(rnd(0.0, std::f32::consts::TAU)) * rnd(0.0, maxoff);
        let mut c1 = self.c0 + off;
        c1.x = c1.x.clamp(rn, MAP - rn);
        c1.y = c1.y.clamp(rn, MAP - rn);
        self.c1 = c1;
        self.r1 = rn;
    }

    pub fn update(&mut self, dt: f32) {
        if self.done {
            return;
        }
        self.timer += dt;
        if !self.shrinking {
            if self.timer >= WAIT[self.phase] {
                self.shrinking = true;
                self.timer = 0.0;
            }
        } else {
            let t = (self.timer / SHRINK[self.phase]).min(1.0);
            let e = t * t * (3.0 - 2.0 * t);
            self.r = self.r0 + (self.r1 - self.r0) * e;
            self.c = self.c0 + (self.c1 - self.c0) * e;
            if t >= 1.0 {
                self.phase += 1;
                self.shrinking = false;
                self.timer = 0.0;
                self.r0 = self.r;
                self.c0 = self.c;
                if self.phase >= PHASES {
                    self.done = true;
                } else {
                    self.pick_next();
                }
            }
        }
    }

    pub fn dps(&self) -> f32 {
        DMG[self.phase.min(PHASES - 1)]
    }

    pub fn inside(&self, p: Vec2) -> bool {
        p.distance(self.c) < self.r
    }

    /// Seconds until the zone starts / finishes shrinking.
    pub fn countdown(&self) -> f32 {
        if self.done {
            return 0.0;
        }
        if self.shrinking {
            SHRINK[self.phase] - self.timer
        } else {
            WAIT[self.phase] - self.timer
        }
    }
}

pub struct World {
    pub walls: Vec<Rect>,
    pub rocks: Vec<Rect>,
    pub buildings: Vec<Rect>,
    pub loot: Vec<Loot>,
    pub zone: Zone,
    pub blocked: Vec<bool>,
}

fn building_walls(b: &Rect, out: &mut Vec<Rect>) {
    let t = 12.0;
    let door = 84.0;
    let door_side: i32 = gen_range(0, 4);
    let door2 = (door_side + 2) % 4;
    for s in 0..4 {
        let has_door = s == door_side || s == door2;
        if s == 0 || s == 2 {
            let y = if s == 0 { b.y } else { b.y + b.h - t };
            if has_door {
                let dx = gen_range(b.x + 30.0, b.x + b.w - 30.0 - door);
                out.push(Rect::new(b.x, y, dx - b.x, t));
                out.push(Rect::new(dx + door, y, b.x + b.w - dx - door, t));
            } else {
                out.push(Rect::new(b.x, y, b.w, t));
            }
        } else {
            let x = if s == 3 { b.x } else { b.x + b.w - t };
            if has_door {
                let dy = gen_range(b.y + 30.0, b.y + b.h - 30.0 - door);
                out.push(Rect::new(x, b.y, t, dy - b.y));
                out.push(Rect::new(x, dy + door, t, b.y + b.h - dy - door));
            } else {
                out.push(Rect::new(x, b.y, t, b.h));
            }
        }
    }
}

fn roll_loot() -> LootKind {
    let r: f32 = gen_range(0.0, 100.0);
    if r < 12.0 {
        LootKind::Weapon(WKind::Pistol)
    } else if r < 26.0 {
        LootKind::Weapon(WKind::Smg)
    } else if r < 40.0 {
        LootKind::Weapon(WKind::Ar)
    } else if r < 49.0 {
        LootKind::Weapon(WKind::Shotgun)
    } else if r < 56.0 {
        LootKind::Weapon(WKind::Sniper)
    } else if r < 78.0 {
        LootKind::Armor
    } else {
        LootKind::Medkit
    }
}

impl World {
    pub fn generate() -> World {
        let mut walls: Vec<Rect> = Vec::new();
        let mut buildings: Vec<Rect> = Vec::new();
        let t = 100.0;
        walls.push(Rect::new(-t, -t, MAP + 2.0 * t, t));
        walls.push(Rect::new(-t, MAP, MAP + 2.0 * t, t));
        walls.push(Rect::new(-t, -t, t, MAP + 2.0 * t));
        walls.push(Rect::new(MAP, -t, t, MAP + 2.0 * t));

        let place = |buildings: &mut Vec<Rect>, p: Vec2| {
            let w = gen_range(140.0, 260.0);
            let h = gen_range(120.0, 220.0);
            let r = Rect::new(p.x - w / 2.0, p.y - h / 2.0, w, h);
            if r.x < 150.0 || r.y < 150.0 || r.x + r.w > MAP - 150.0 || r.y + r.h > MAP - 150.0 {
                return false;
            }
            for b in buildings.iter() {
                let infl = Rect::new(b.x - 70.0, b.y - 70.0, b.w + 140.0, b.h + 140.0);
                if infl.overlaps(&r) {
                    return false;
                }
            }
            buildings.push(r);
            true
        };

        for gx in 0..3 {
            for gy in 0..3 {
                let c = vec2(
                    500.0 + gx as f32 * 1100.0 + gen_range(-150.0, 150.0),
                    500.0 + gy as f32 * 1100.0 + gen_range(-150.0, 150.0),
                );
                let n: i32 = gen_range(4, 8);
                let mut placed = 0;
                for _ in 0..50 {
                    if placed >= n {
                        break;
                    }
                    let p = c + vec2(gen_range(-380.0, 380.0), gen_range(-380.0, 380.0));
                    if place(&mut buildings, p) {
                        placed += 1;
                    }
                }
            }
        }
        for _ in 0..40 {
            let p = vec2(gen_range(200.0, MAP - 200.0), gen_range(200.0, MAP - 200.0));
            place(&mut buildings, p);
        }
        for b in &buildings {
            building_walls(b, &mut walls);
        }

        let mut rocks: Vec<Rect> = Vec::new();
        for _ in 0..90 {
            let w = gen_range(40.0, 90.0);
            let h = gen_range(40.0, 90.0);
            let r = Rect::new(gen_range(100.0, MAP - 200.0), gen_range(100.0, MAP - 200.0), w, h);
            let infl = Rect::new(r.x - 50.0, r.y - 50.0, r.w + 100.0, r.h + 100.0);
            if buildings.iter().any(|b| b.overlaps(&infl)) || rocks.iter().any(|k| k.overlaps(&infl)) {
                continue;
            }
            rocks.push(r);
        }
        walls.extend(rocks.iter().cloned());

        // Navigation grid
        let mut blocked = vec![false; GN * GN];
        for gy in 0..GN {
            for gx in 0..GN {
                let p = vec2((gx as f32 + 0.5) * CELL, (gy as f32 + 0.5) * CELL);
                blocked[gy * GN + gx] = walls.iter().any(|w| dist_point_rect(p, w) < 18.0);
            }
        }

        let mut w = World { walls, rocks, buildings, loot: Vec::new(), zone: Zone::new(), blocked };

        // Loot inside buildings
        let bl = w.buildings.clone();
        for b in &bl {
            let n: i32 = gen_range(2, 4);
            for _ in 0..n {
                let pos = vec2(
                    gen_range(b.x + 30.0, b.x + b.w - 30.0),
                    gen_range(b.y + 30.0, b.y + b.h - 30.0),
                );
                w.loot.push(Loot { pos, kind: roll_loot(), taken: false });
            }
        }
        // Loot scattered outside
        let mut tries = 0;
        let mut added = 0;
        while added < 50 && tries < 500 {
            tries += 1;
            let p = vec2(gen_range(100.0, MAP - 100.0), gen_range(100.0, MAP - 100.0));
            if !w.blocked_at(p) {
                w.loot.push(Loot { pos: p, kind: roll_loot(), taken: false });
                added += 1;
            }
        }
        w
    }

    pub fn blocked_at(&self, p: Vec2) -> bool {
        let gx = (p.x / CELL) as i32;
        let gy = (p.y / CELL) as i32;
        if gx < 0 || gy < 0 || gx >= GN as i32 || gy >= GN as i32 {
            return true;
        }
        self.blocked[gy as usize * GN + gx as usize]
    }

    /// Push a circle out of any walls it overlaps.
    pub fn resolve(&self, mut pos: Vec2, r: f32) -> Vec2 {
        for _ in 0..2 {
            for w in &self.walls {
                if let Some(push) = circle_rect_push(pos, r, w) {
                    pos += push;
                }
            }
        }
        pos
    }

    /// Clear line of sight (no wall crosses the segment).
    pub fn los(&self, a: Vec2, b: Vec2) -> bool {
        let (minx, maxx) = (a.x.min(b.x), a.x.max(b.x));
        let (miny, maxy) = (a.y.min(b.y), a.y.max(b.y));
        for w in &self.walls {
            if w.x > maxx || w.x + w.w < minx || w.y > maxy || w.y + w.h < miny {
                continue;
            }
            if seg_hits_rect(a, b, w) {
                return false;
            }
        }
        true
    }

    /// Like `los` but with margin, so a body can actually walk the line.
    pub fn clear_wide(&self, a: Vec2, b: Vec2) -> bool {
        let m = 15.0;
        let (minx, maxx) = (a.x.min(b.x) - m, a.x.max(b.x) + m);
        let (miny, maxy) = (a.y.min(b.y) - m, a.y.max(b.y) + m);
        for w in &self.walls {
            if w.x > maxx || w.x + w.w < minx || w.y > maxy || w.y + w.h < miny {
                continue;
            }
            let r = Rect::new(w.x - m, w.y - m, w.w + 2.0 * m, w.h + 2.0 * m);
            if seg_hits_rect(a, b, &r) {
                return false;
            }
        }
        true
    }

    fn cell_idx(p: Vec2) -> usize {
        let gx = ((p.x / CELL) as i32).clamp(0, GN as i32 - 1) as usize;
        let gy = ((p.y / CELL) as i32).clamp(0, GN as i32 - 1) as usize;
        gy * GN + gx
    }

    fn free_cell(&self, p: Vec2) -> usize {
        let i = Self::cell_idx(p);
        if !self.blocked[i] {
            return i;
        }
        let (cx, cy) = ((i % GN) as i32, (i / GN) as i32);
        for r in 1..8i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx < 0 || ny < 0 || nx >= GN as i32 || ny >= GN as i32 {
                        continue;
                    }
                    let ni = ny as usize * GN + nx as usize;
                    if !self.blocked[ni] {
                        return ni;
                    }
                }
            }
        }
        i
    }

    fn cell_center(i: usize) -> Vec2 {
        vec2(((i % GN) as f32 + 0.5) * CELL, ((i / GN) as f32 + 0.5) * CELL)
    }

    /// A* over the nav grid, followed by string-pulling into a few waypoints.
    pub fn path(&self, from: Vec2, to: Vec2) -> Vec<Vec2> {
        let s = self.free_cell(from);
        let g = self.free_cell(to);
        if s == g {
            return vec![to];
        }
        let n = GN * GN;
        let mut gs = vec![f32::MAX; n];
        let mut came = vec![usize::MAX; n];
        let mut open: BinaryHeap<(Reverse<i32>, usize)> = BinaryHeap::new();
        let h = |i: usize| Self::cell_center(i).distance(Self::cell_center(g)) / CELL;
        gs[s] = 0.0;
        open.push((Reverse((h(s) * 10.0) as i32), s));
        let dirs: [(i32, i32); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)];
        let mut iters = 0;
        while let Some((_, cur)) = open.pop() {
            if cur == g {
                break;
            }
            iters += 1;
            if iters > 6000 {
                break;
            }
            let (cx, cy) = ((cur % GN) as i32, (cur / GN) as i32);
            for (dx, dy) in dirs {
                let (nx, ny) = (cx + dx, cy + dy);
                if nx < 0 || ny < 0 || nx >= GN as i32 || ny >= GN as i32 {
                    continue;
                }
                let ni = ny as usize * GN + nx as usize;
                if self.blocked[ni] {
                    continue;
                }
                let diag = dx != 0 && dy != 0;
                if diag
                    && (self.blocked[cy as usize * GN + nx as usize]
                        || self.blocked[ny as usize * GN + cx as usize])
                {
                    continue;
                }
                let ng = gs[cur] + if diag { 1.414 } else { 1.0 };
                if ng < gs[ni] {
                    gs[ni] = ng;
                    came[ni] = cur;
                    open.push((Reverse(((ng + h(ni)) * 10.0) as i32), ni));
                }
            }
        }
        if came[g] == usize::MAX {
            return vec![to];
        }
        let mut cells = vec![g];
        let mut cur = g;
        while cur != s {
            cur = came[cur];
            cells.push(cur);
        }
        cells.reverse();
        let pts: Vec<Vec2> = cells.iter().map(|&i| Self::cell_center(i)).collect();

        let mut out = Vec::new();
        let mut i = 1;
        let mut cur_pos = from;
        while i < pts.len() {
            let mut j = (i + 10).min(pts.len() - 1);
            while j > i && !self.clear_wide(cur_pos, pts[j]) {
                j -= 1;
            }
            out.push(pts[j]);
            cur_pos = pts[j];
            i = j + 1;
        }
        if out.is_empty() {
            out.push(to);
        }
        out
    }
}
