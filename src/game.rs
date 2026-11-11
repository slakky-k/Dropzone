use crate::bot::*;
use crate::fighter::*;
use crate::input::*;
use crate::util::*;
use crate::weapons::*;
use crate::world::*;
use macroquad::prelude::*;

pub const NBOTS: usize = 23;
pub const PLAYER_NAME: &str = "RIVET";
const NAMES: [&str; NBOTS] = [
    "Viper", "Nova", "Ghost", "Rook", "Echo", "Blitz", "Sable", "Kestrel", "Onyx", "Drift", "Halo", "Cinder",
    "Mako", "Pixel", "Raven", "Zephyr", "Talon", "Juno", "Atlas", "Wraith", "Flint", "Orbit", "Quill",
];

pub struct Spark {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub max: f32,
    pub color: Color,
}

pub struct Game {
    pub w: World,
    pub fs: Vec<Fighter>,
    pub brains: Vec<Option<Brain>>,
    pub bullets: Vec<Bullet>,
    pub sounds: Vec<Sound>,
    pub sparks: Vec<Spark>,
    pub feed: Vec<(String, f32)>,
    pub cam: Vec2,
    pub shake: f32,
    pub shake_off: Vec2,
    pub t: f32,
    pub over: Option<bool>,
    pub place: usize,
    pub hitmark: f32,
    pub hit_kill: bool,
    pub hurt_t: f32,
    pub notice: (String, f32),
    pub ctl: Controls,
    zone_tick: f32,
}

impl Game {
    pub fn new(ctl: Controls) -> Game {
        let w = World::generate();
        let mut spawns: Vec<Vec2> = Vec::new();
        let mut tries = 0;
        while spawns.len() < NBOTS + 1 && tries < 5000 {
            tries += 1;
            let p = vec2(rnd(150.0, MAP - 150.0), rnd(150.0, MAP - 150.0));
            if w.blocked_at(p) || spawns.iter().any(|s| s.distance(p) < 350.0) {
                continue;
            }
            spawns.push(p);
        }
        while spawns.len() < NBOTS + 1 {
            let n = spawns.len() as f32;
            spawns.push(vec2(MAP / 2.0, MAP / 2.0) + dir(n) * 100.0);
        }
        let mut fs = vec![Fighter::new(0, PLAYER_NAME, spawns[0], false)];
        let mut brains: Vec<Option<Brain>> = vec![None];
        for k in 0..NBOTS {
            let mut f = Fighter::new(k + 1, NAMES[k], spawns[k + 1], true);
            f.aim = rnd(0.0, std::f32::consts::TAU);
            fs.push(f);
            brains.push(Some(Brain::new(rnd(0.3, 0.95), spawns[k + 1])));
        }
        Game {
            w, cam: spawns[0], fs, brains, bullets: vec![], sounds: vec![], sparks: vec![], feed: vec![],
            shake: 0.0, shake_off: Vec2::ZERO, t: 0.0, over: None, place: 0, hitmark: 0.0, hit_kill: false,
            hurt_t: 0.0, notice: (String::new(), 0.0), ctl, zone_tick: 0.0,
        }
    }

    pub fn alive_count(&self) -> usize {
        self.fs.iter().filter(|f| f.alive).count()
    }

    /// Offset added to world coordinates to get screen coordinates.
    pub fn view(&self) -> Vec2 {
        vec2(screen_width() / 2.0, screen_height() / 2.0) - self.cam + self.shake_off
    }

    fn feed_push(&mut self, s: String) {
        self.feed.push((s, 5.0));
        if self.feed.len() > 5 {
            self.feed.remove(0);
        }
    }

    fn burst(&mut self, pos: Vec2, color: Color, n: usize) {
        for _ in 0..n {
            let a = rnd(0.0, std::f32::consts::TAU);
            let s = rnd(60.0, 240.0);
            let life = rnd(0.15, 0.4);
            self.sparks.push(Spark { pos, vel: dir(a) * s, life, max: life, color });
        }
    }

    fn kill(&mut self, victim: usize, killer: Option<usize>) {
        let nv = self.fs[victim].name.clone();
        let msg = match killer {
            Some(k) => format!("{} eliminated {}", self.fs[k].name, nv),
            None => format!("{} was lost to the storm", nv),
        };
        if let Some(k) = killer {
            self.fs[k].kills += 1;
        }
        self.feed_push(msg);
        let pos = self.fs[victim].pos;
        let kinds: Vec<WKind> = self.fs[victim].guns.iter().flatten().map(|g| g.kind).filter(|k| k.spec().score > 1).collect();
        for (n, k) in kinds.iter().enumerate() {
            let p = self.w.resolve(pos + dir(n as f32 * 3.0) * 22.0, 10.0);
            self.w.loot.push(Loot { pos: p, kind: LootKind::Weapon(*k), taken: false });
        }
        for n in 0..self.fs[victim].medkits {
            let p = self.w.resolve(pos + dir(2.0 + n as f32) * 24.0, 10.0);
            self.w.loot.push(Loot { pos: p, kind: LootKind::Medkit, taken: false });
        }
        if self.fs[victim].armor > 20.0 {
            let p = self.w.resolve(pos + dir(5.0) * 24.0, 10.0);
            self.w.loot.push(Loot { pos: p, kind: LootKind::Armor, taken: false });
        }
        self.burst(pos, Color::new(1.0, 0.3, 0.2, 1.0), 24);
        if victim == 0 {
            self.over = Some(false);
            self.place = self.alive_count() + 1;
        } else if self.fs[0].alive && self.alive_count() == 1 {
            self.over = Some(true);
            self.place = 1;
        }
    }

    fn assist_angle(&self) -> Option<f32> {
        let me = &self.fs[0];
        let mut best: Option<(f32, f32)> = None;
        for o in self.fs.iter().skip(1) {
            if !o.alive {
                continue;
            }
            let d = o.pos.distance(me.pos);
            if d < 650.0 && self.w.los(me.pos, o.pos) && best.map_or(true, |(bd, _)| d < bd) {
                best = Some((d, angle_of(o.pos - me.pos)));
            }
        }
        best.map(|(_, a)| a)
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.min(0.05);
        self.t += dt;
        self.w.zone.update(dt);
        for s in self.sounds.iter_mut() {
            s.age += dt;
        }
        self.sounds.retain(|s| s.age < 1.0);

        // Player
        if self.fs[0].alive {
            let assist = self.assist_angle();
            let ps = self.fs[0].pos + self.view();
            let mut it = self.ctl.read(ps, assist);
            if it.swap == Some(2) {
                it.swap = Some(1 - self.fs[0].cur);
            }
            let before = self.fs[0].flash;
            self.fs[0].update(&it, dt, &self.w, &mut self.bullets, &mut self.sounds);
            if self.fs[0].flash > before {
                self.shake = self.shake.max(0.12);
            }
        }

        // Bots
        for i in 1..self.fs.len() {
            if !self.fs[i].alive {
                continue;
            }
            let it = self.brains[i].as_mut().unwrap().think(i, &self.fs, &self.w, &self.sounds, dt);
            self.fs[i].update(&it, dt, &self.w, &mut self.bullets, &mut self.sounds);
        }

        // Bullets
        let mut i = 0;
        while i < self.bullets.len() {
            let (a, b, owner, dmg) = {
                let bl = &mut self.bullets[i];
                let a = bl.pos;
                bl.pos += bl.vel * dt;
                bl.life -= dt;
                (a, bl.pos, bl.owner, bl.dmg)
            };
            let mut remove = self.bullets[i].life <= 0.0;
            let mut hit: Option<usize> = None;
            for f in &self.fs {
                if !f.alive || f.id == owner {
                    continue;
                }
                if closest_on_seg(a, b, f.pos).distance(f.pos) < RADIUS + 2.0 && self.w.los(a, f.pos) {
                    hit = Some(f.id);
                    break;
                }
            }
            if let Some(j) = hit {
                let died = self.fs[j].take_damage(dmg);
                let p = self.fs[j].pos;
                self.burst(p, Color::new(1.0, 0.25, 0.2, 1.0), 5);
                if owner == 0 {
                    self.hitmark = 0.18;
                    self.hit_kill = died;
                }
                if j == 0 {
                    self.shake = self.shake.max(0.35);
                    self.hurt_t = 0.4;
                }
                if died {
                    self.kill(j, Some(owner));
                }
                remove = true;
            } else if !remove && self.w.walls.iter().any(|r| seg_hits_rect(a, b, r)) {
                self.burst(b, Color::new(1.0, 0.85, 0.5, 1.0), 3);
                remove = true;
            }
            if remove {
                self.bullets.swap_remove(i);
            } else {
                i += 1;
            }
        }

        // Storm damage
        self.zone_tick += dt;
        if self.zone_tick >= 0.5 {
            self.zone_tick = 0.0;
            let dps = self.w.zone.dps();
            for j in 0..self.fs.len() {
                if self.fs[j].alive && !self.w.zone.inside(self.fs[j].pos) {
                    let died = self.fs[j].take_damage(dps * 0.5);
                    if j == 0 {
                        self.hurt_t = 0.2;
                    }
                    if died {
                        self.kill(j, None);
                    }
                }
            }
        }

        // Pickups
        for fi in 0..self.fs.len() {
            if !self.fs[fi].alive {
                continue;
            }
            let p = self.fs[fi].pos;
            for li in 0..self.w.loot.len() {
                let (taken, lp, kind) = {
                    let l = &self.w.loot[li];
                    (l.taken, l.pos, l.kind)
                };
                if taken || lp.distance(p) > 26.0 || !Brain::wants(&self.fs[fi], kind) {
                    continue;
                }
                let f = &mut self.fs[fi];
                let (ok, label) = match kind {
                    LootKind::Weapon(k) => (f.pick_weapon(k), k.spec().name.to_string()),
                    LootKind::Armor => {
                        f.armor = (f.armor + 50.0).min(100.0);
                        (true, "Armor plate".to_string())
                    }
                    LootKind::Medkit => {
                        f.medkits += 1;
                        (true, "Medkit".to_string())
                    }
                };
                if ok {
                    self.w.loot[li].taken = true;
                    if fi == 0 {
                        self.notice = (format!("Picked up {}", label), 2.0);
                    }
                }
            }
        }

        // Effects, camera
        for s in self.sparks.iter_mut() {
            s.pos += s.vel * dt;
            s.vel *= 0.92;
            s.life -= dt;
        }
        self.sparks.retain(|s| s.life > 0.0);
        for f in self.feed.iter_mut() {
            f.1 -= dt;
        }
        self.feed.retain(|f| f.1 > 0.0);
        self.notice.1 -= dt;
        self.hitmark = (self.hitmark - dt).max(0.0);
        self.hurt_t = (self.hurt_t - dt).max(0.0);
        self.shake = (self.shake - dt * 1.8).max(0.0);
        self.shake_off = vec2(rnd(-1.0, 1.0), rnd(-1.0, 1.0)) * self.shake * 9.0;

        let me = &self.fs[0];
        let target = me.pos + dir(me.aim) * 70.0 + me.vel * 0.18;
        let k = 1.0 - (-7.0 * dt).exp();
        self.cam += (target - self.cam) * k;
    }
}
