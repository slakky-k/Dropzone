use crate::fighter::*;
use crate::util::*;
use crate::weapons::*;
use crate::world::*;
use macroquad::prelude::*;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Loot,
    Rotate,
    Engage,
    Cover,
    Hunt,
    Investigate,
}

/// Human-like bot brain. `skill` (0..1) scales reaction time, turn speed,
/// aim noise, target leading and tactical choices.
pub struct Brain {
    pub state: State,
    pub skill: f32,
    pub aggr: f32,
    target: Option<usize>,
    last_seen: Vec2,
    since_seen: f32,
    react: f32,
    path: Vec<Vec2>,
    repath: f32,
    goal: Vec2,
    strafe_dir: f32,
    strafe_t: f32,
    stuck_t: f32,
    last_pos: Vec2,
    unstick_t: f32,
    unstick_dir: Vec2,
    aim_err: f32,
    err_t: f32,
    burst_t: f32,
    burst_pause: f32,
    cover: Option<Vec2>,
    cover_t: f32,
    heard: Option<Vec2>,
    heard_cd: f32,
    rotate_key: i32,
    rotate_goal: Vec2,
    loot_goal: Option<usize>,
    loot_t: f32,
    slide_roll: f32,
    swap_cd: f32,
    flank_side: f32,
}

impl Brain {
    pub fn new(skill: f32, pos: Vec2) -> Self {
        Brain {
            state: State::Loot, skill, aggr: rnd(0.2, 0.95),
            target: None, last_seen: pos, since_seen: 99.0, react: 0.0,
            path: Vec::new(), repath: 0.0, goal: pos,
            strafe_dir: 1.0, strafe_t: 0.0, stuck_t: 0.0, last_pos: pos,
            unstick_t: 0.0, unstick_dir: Vec2::X, aim_err: 0.0, err_t: 0.0,
            burst_t: 0.5, burst_pause: 0.0, cover: None, cover_t: 0.0,
            heard: None, heard_cd: 0.0, rotate_key: -1, rotate_goal: pos,
            loot_goal: None, loot_t: 0.0, slide_roll: 2.0, swap_cd: 0.0,
            flank_side: 1.0,
        }
    }

    fn goto(&mut self, me: &Fighter, goal: Vec2, w: &World, dt: f32) -> Vec2 {
        self.repath -= dt;
        let need = self.repath <= 0.0
            || goal.distance(self.goal) > 120.0
            || (self.path.is_empty() && me.pos.distance(goal) > 40.0);
        if need {
            self.path = w.path(me.pos, goal);
            self.goal = goal;
            self.repath = rnd(0.7, 1.2);
        }
        while let Some(p) = self.path.first() {
            if p.distance(me.pos) < 28.0 {
                self.path.remove(0);
            } else {
                break;
            }
        }
        match self.path.first() {
            Some(p) => (*p - me.pos).normalize_or_zero(),
            None => Vec2::ZERO,
        }
    }

    pub fn wants(me: &Fighter, k: LootKind) -> bool {
        match k {
            LootKind::Weapon(wk) => {
                if me.guns[1].is_none() {
                    return me.guns[0].as_ref().map(|g| g.kind) != Some(wk);
                }
                let min = me.guns.iter().flatten().map(|g| g.kind.spec().score).min().unwrap_or(0);
                wk.spec().score > min && !me.guns.iter().flatten().any(|g| g.kind == wk)
            }
            LootKind::Armor => me.armor < 80.0,
            LootKind::Medkit => me.medkits < 3,
        }
    }

    fn find_cover(&self, me: &Fighter, threat: Vec2, w: &World) -> Option<Vec2> {
        let mut best: Option<(f32, Vec2)> = None;
        for k in 0..16 {
            let a = k as f32 * std::f32::consts::TAU / 16.0;
            for r in [70.0, 120.0, 180.0] {
                let p = me.pos + dir(a) * r;
                if w.blocked_at(p) || w.los(p, threat) || !w.clear_wide(me.pos, p) || !w.zone.inside(p) {
                    continue;
                }
                let score = r - p.distance(threat) * 0.1;
                if best.map_or(true, |(s, _)| score < s) {
                    best = Some((score, p));
                }
            }
        }
        best.map(|(_, p)| p)
    }

    pub fn think(&mut self, i: usize, fs: &[Fighter], w: &World, sounds: &[Sound], dt: f32) -> Intent {
        let me = &fs[i];
        let mut it = Intent::default();
        it.aim = me.aim;
        self.since_seen += dt;
        self.heard_cd -= dt;
        self.strafe_t -= dt;
        self.err_t -= dt;
        self.slide_roll -= dt;
        self.swap_cd -= dt;
        self.cover_t -= dt;
        self.loot_t -= dt;
        if self.react > 0.0 {
            self.react -= dt;
        }
        if let Some(t) = self.target {
            if !fs[t].alive {
                self.target = None;
                self.since_seen = 99.0;
                self.cover = None;
            }
        }

        // ---------- Perception ----------
        let mut best: Option<(usize, f32)> = None;
        for (j, o) in fs.iter().enumerate() {
            if j == i || !o.alive {
                continue;
            }
            let d = o.pos - me.pos;
            let dist = d.length();
            let mut vr = 620.0;
            if o.vel.length() > 200.0 {
                vr *= 1.25;
            }
            if o.flash > 0.0 {
                vr = 900.0;
            }
            if dist > vr {
                continue;
            }
            let in_fov = angle_diff(angle_of(d), me.aim).abs() < 1.9 || dist < 140.0;
            if !in_fov || !w.los(me.pos, o.pos) {
                continue;
            }
            if best.map_or(true, |(_, bd)| dist < bd) {
                best = Some((j, dist));
            }
        }
        let mut visible: Option<usize> = None;
        if let Some((j, _)) = best {
            if self.target != Some(j) {
                self.target = Some(j);
                self.react = lerp(0.55, 0.15, self.skill) + rnd(0.0, 0.1);
                self.cover = None;
                self.flank_side = if rnd(0.0, 1.0) < 0.5 { 1.0 } else { -1.0 };
            }
            self.last_seen = fs[j].pos;
            self.since_seen = 0.0;
            visible = Some(j);
        } else if self.since_seen > 8.0 {
            self.target = None;
        }

        // Hearing gunfire: third-party fights
        if visible.is_none() && self.heard_cd <= 0.0 {
            for s in sounds {
                if s.owner == i || s.age > 0.4 {
                    continue;
                }
                let dd = s.pos.distance(me.pos);
                if dd < 900.0 && dd > 120.0 {
                    self.heard_cd = rnd(2.0, 4.0);
                    if rnd(0.0, 1.0) < self.aggr && me.hp > 50.0 {
                        self.heard = Some(s.pos);
                    }
                    break;
                }
            }
        }

        // ---------- Situation ----------
        let z = &w.zone;
        let out_now = me.pos.distance(z.c) > z.r - 25.0;
        let out_next = me.pos.distance(z.c1) > z.r1 - 80.0;
        let urgent = out_now || (out_next && (z.shrinking || z.countdown() < 25.0));
        let has_main = me.guns.iter().flatten().any(|g| g.kind.spec().score >= 3);
        let hp_low = me.hp + me.armor * 0.5 < 40.0 && me.medkits > 0;
        let dist_t = visible.map(|j| fs[j].pos.distance(me.pos)).unwrap_or(f32::MAX);

        let mut state;
        if me.healing() {
            state = State::Cover;
        } else if visible.is_some() {
            state = if hp_low && dist_t > 150.0 { State::Cover } else { State::Engage };
        } else if self.target.is_some() && self.since_seen < 6.0 {
            state = if hp_low || (self.aggr < 0.4 && me.hp < 70.0) { State::Cover } else { State::Hunt };
        } else if self.heard.is_some() {
            state = State::Investigate;
        } else if urgent {
            state = State::Rotate;
        } else {
            state = State::Loot;
        }
        if urgent && state != State::Engage && dist_t > 300.0 && !me.healing() {
            state = State::Rotate;
        }
        self.state = state;

        let mut aim_set = false;

        match state {
            State::Engage => {
                let j = visible.unwrap();
                let o = &fs[j];
                let to = o.pos - me.pos;
                let d = to.length();
                let gun = me.gun();
                let (pref, bspeed, range, auto) = match gun {
                    Some(g) => {
                        let s = g.kind.spec();
                        (s.pref_range, s.speed, s.range, s.auto)
                    }
                    None => (200.0, 1000.0, 500.0, false),
                };

                // Aim: lead the target, add skill-dependent noise, turn at a limited rate
                let lead = lerp(0.3, 1.0, self.skill);
                let tp = o.pos + o.vel * (d / bspeed) * lead;
                if self.err_t <= 0.0 {
                    self.aim_err = rnd(-1.0, 1.0) * (1.0 - self.skill) * 0.10;
                    self.err_t = rnd(0.2, 0.5);
                }
                let moving_mul = if o.vel.length() > 150.0 { 1.5 } else { 1.0 };
                let want_a = angle_of(tp - me.pos) + self.aim_err * moving_mul;
                let turn = lerp(5.0, 15.0, self.skill) * dt;
                let diff = angle_diff(want_a, me.aim);
                it.aim = me.aim + diff.clamp(-turn, turn);
                aim_set = true;

                // Fire discipline: bursts for automatics
                let tol = (14.0 / d.max(40.0)).atan() * 1.3 + 0.01;
                let aimed = angle_diff(want_a, it.aim).abs() < tol;
                if self.react <= 0.0 && aimed && d < range * 0.95 {
                    if auto {
                        if self.burst_pause > 0.0 {
                            self.burst_pause -= dt;
                        } else {
                            it.fire = true;
                            self.burst_t -= dt;
                            if self.burst_t <= 0.0 {
                                self.burst_t = rnd(0.35, 0.9) * (0.8 + self.skill * 0.4);
                                self.burst_pause = rnd(0.08, 0.35) * (1.2 - self.skill);
                            }
                        }
                    } else {
                        it.fire = true;
                    }
                }

                // Movement: strafe, hold preferred range, avoid walls
                let n = to.normalize_or_zero();
                let perp = vec2(-n.y, n.x);
                if self.strafe_t <= 0.0 {
                    self.strafe_dir = if rnd(0.0, 1.0) < 0.5 { 1.0 } else { -1.0 };
                    self.strafe_t = rnd(0.4, 1.4);
                }
                let radial = ((d - pref) / 120.0).clamp(-1.0, 1.0);
                let mut mv = perp * self.strafe_dir * 0.85 + n * radial;
                if w.blocked_at(me.pos + mv.normalize_or_zero() * 45.0) {
                    self.strafe_dir = -self.strafe_dir;
                    self.strafe_t = rnd(0.4, 1.0);
                    mv = perp * self.strafe_dir * 0.85 + n * radial;
                }
                it.mv = mv;
                it.sprint = d > pref * 1.5 && self.skill > 0.4;
                if self.slide_roll <= 0.0 {
                    self.slide_roll = rnd(1.5, 3.5);
                    if d > 200.0 && d < 420.0 && me.vel.length() > 170.0 && self.skill > 0.5 && rnd(0.0, 1.0) < 0.5 {
                        it.slide = true;
                    }
                }

                // Weapon management
                if let Some(g) = gun {
                    if g.ammo * 4 <= g.kind.spec().mag && g.can_reload() && d > 200.0 {
                        it.reload = true;
                    }
                    if g.ammo == 0 {
                        let other = 1 - me.cur;
                        if me.guns[other].as_ref().map_or(false, |og| og.ammo > 0) {
                            it.swap = Some(other);
                        }
                    } else if self.swap_cd <= 0.0 {
                        let other = 1 - me.cur;
                        if let Some(og) = &me.guns[other] {
                            let cur_fit = (g.kind.spec().pref_range - d).abs();
                            let oth_fit = (og.kind.spec().pref_range - d).abs();
                            if og.ammo > 0 && oth_fit + 150.0 < cur_fit {
                                it.swap = Some(other);
                                self.swap_cd = 3.0;
                            }
                        }
                    }
                }
            }

            State::Cover => {
                let threat = self.last_seen;
                if self.cover.is_none() || self.cover_t <= 0.0 {
                    if !me.healing() {
                        self.cover = self.find_cover(me, threat, w);
                    }
                    self.cover_t = 1.5;
                }
                match self.cover {
                    Some(c) if me.pos.distance(c) > 20.0 && !me.healing() => {
                        it.mv = self.goto(me, c, w, dt);
                        it.sprint = true;
                    }
                    Some(_) => {
                        if me.hp < 100.0 && me.medkits > 0 && visible.is_none() {
                            it.heal = true;
                        } else if me.gun().map_or(false, |g| g.can_reload()) {
                            it.reload = true;
                        }
                        let a = angle_of(threat - me.pos);
                        let diff = angle_diff(a, me.aim);
                        it.aim = me.aim + diff.clamp(-6.0 * dt, 6.0 * dt);
                        aim_set = true;
                    }
                    None => {
                        it.mv = -(threat - me.pos).normalize_or_zero();
                        it.sprint = true;
                        if me.healing() {
                            it.mv = Vec2::ZERO;
                        }
                    }
                }
            }

            State::Hunt => {
                let n = (self.last_seen - me.pos).normalize_or_zero();
                let perp = vec2(-n.y, n.x);
                let mut goal = self.last_seen;
                if self.skill > 0.6 && me.pos.distance(self.last_seen) > 200.0 {
                    goal += perp * 150.0 * self.flank_side; // flank instead of walking into the barrel
                }
                it.mv = self.goto(me, goal, w, dt);
                it.sprint = self.skill > 0.3;
                if me.gun().map_or(false, |g| g.can_reload() && g.ammo * 2 < g.kind.spec().mag) {
                    it.reload = true;
                }
            }

            State::Investigate => {
                let h = self.heard.unwrap();
                it.mv = self.goto(me, h, w, dt);
                it.sprint = self.skill < 0.6;
                if me.pos.distance(h) < 150.0 || self.heard_cd < -6.0 {
                    self.heard = None;
                }
            }

            State::Loot => {
                if me.hp < 70.0 && me.medkits > 0 && self.since_seen > 4.0 {
                    it.heal = true;
                } else {
                    // choose a loot target
                    let need_loot = self.loot_goal.map_or(true, |k| w.loot[k].taken || !Self::wants(me, w.loot[k].kind));
                    if need_loot || self.loot_t <= 0.0 {
                        self.loot_t = 1.5;
                        let mut bestl: Option<(f32, usize)> = None;
                        for (k, l) in w.loot.iter().enumerate() {
                            if l.taken || !Self::wants(me, l.kind) {
                                continue;
                            }
                            let d = l.pos.distance(me.pos);
                            if d > 1100.0 || l.pos.distance(z.c1) > z.r1 + 150.0 {
                                continue;
                            }
                            let mut score = d;
                            if !has_main && matches!(l.kind, LootKind::Weapon(_)) {
                                score -= 400.0;
                            }
                            if me.hp < 60.0 && l.kind == LootKind::Medkit {
                                score -= 300.0;
                            }
                            if bestl.map_or(true, |(s, _)| score < s) {
                                bestl = Some((score, k));
                            }
                        }
                        self.loot_goal = bestl.map(|(_, k)| k);
                    }
                    match self.loot_goal {
                        Some(k) => {
                            it.mv = self.goto(me, w.loot[k].pos, w, dt);
                            it.sprint = true;
                        }
                        None => {
                            // nothing worth grabbing: drift towards the next circle
                            let key = z.phase as i32 * 2 + z.shrinking as i32;
                            if self.rotate_key != key {
                                self.rotate_key = key;
                                let mut g = z.c1 + dir(rnd(0.0, std::f32::consts::TAU)) * rnd(0.0, z.r1 * 0.6);
                                g.x = g.x.clamp(60.0, MAP - 60.0);
                                g.y = g.y.clamp(60.0, MAP - 60.0);
                                self.rotate_goal = g;
                            }
                            it.mv = self.goto(me, self.rotate_goal, w, dt);
                        }
                    }
                }
            }

            State::Rotate => {
                let key = z.phase as i32 * 2 + z.shrinking as i32;
                if self.rotate_key != key {
                    self.rotate_key = key;
                    let mut g = z.c1 + dir(rnd(0.0, std::f32::consts::TAU)) * rnd(0.0, z.r1 * 0.6);
                    g.x = g.x.clamp(60.0, MAP - 60.0);
                    g.y = g.y.clamp(60.0, MAP - 60.0);
                    self.rotate_goal = g;
                }
                it.mv = self.goto(me, self.rotate_goal, w, dt);
                it.sprint = true;
                if me.pos.distance(self.rotate_goal) < 60.0 {
                    it.mv = Vec2::ZERO;
                }
            }
        }

        // Anti-stuck
        self.stuck_t += dt;
        if self.stuck_t >= 0.6 {
            let moved = me.pos.distance(self.last_pos);
            if moved < 10.0 && it.mv.length() > 0.3 && !me.healing() {
                self.unstick_t = 0.7;
                self.unstick_dir = dir(rnd(0.0, std::f32::consts::TAU));
                self.repath = 0.0;
            }
            self.last_pos = me.pos;
            self.stuck_t = 0.0;
        }
        if self.unstick_t > 0.0 {
            self.unstick_t -= dt;
            it.mv = self.unstick_dir;
        }

        // Look where we're going when not aiming at something
        if !aim_set {
            let a = if it.mv.length() > 0.1 { angle_of(it.mv) } else { me.aim };
            let diff = angle_diff(a, me.aim);
            it.aim = me.aim + diff.clamp(-7.0 * dt, 7.0 * dt);
        }
        it
    }
}
