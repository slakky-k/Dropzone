use crate::util::*;
use crate::weapons::*;
use crate::world::*;
use macroquad::prelude::*;

pub const RADIUS: f32 = 14.0;
pub const WALK: f32 = 170.0;
pub const SPRINT: f32 = 265.0;
const SLIDE_TIME: f32 = 0.65;

#[derive(Clone, Default)]
pub struct Intent {
    pub mv: Vec2,
    pub aim: f32,
    pub fire: bool,
    pub reload: bool,
    pub sprint: bool,
    pub slide: bool,
    pub heal: bool,
    pub swap: Option<usize>,
}

pub struct Bullet {
    pub pos: Vec2,
    pub vel: Vec2,
    pub dmg: f32,
    pub owner: usize,
    pub life: f32,
    pub color: Color,
}

pub struct Sound {
    pub pos: Vec2,
    pub age: f32,
    pub owner: usize,
}

pub struct Fighter {
    pub id: usize,
    pub name: String,
    pub is_bot: bool,
    pub pos: Vec2,
    pub vel: Vec2,
    pub hp: f32,
    pub armor: f32,
    pub alive: bool,
    pub aim: f32,
    pub guns: [Option<Gun>; 2],
    pub cur: usize,
    pub slide_t: f32,
    pub slide_cd: f32,
    pub slide_dir: Vec2,
    pub bloom: f32,
    pub heal_t: f32,
    pub medkits: i32,
    pub kills: u32,
    pub flash: f32,
    pub hit_flash: f32,
    pub step: f32,
}

impl Fighter {
    pub fn new(id: usize, name: &str, pos: Vec2, is_bot: bool) -> Self {
        Fighter {
            id, name: name.to_string(), is_bot, pos, vel: Vec2::ZERO,
            hp: 100.0, armor: 0.0, alive: true, aim: 0.0,
            guns: [Some(Gun::new(WKind::Pistol)), None], cur: 0,
            slide_t: 0.0, slide_cd: 0.0, slide_dir: Vec2::X, bloom: 0.0,
            heal_t: 0.0, medkits: 0, kills: 0, flash: 0.0, hit_flash: 0.0, step: 0.0,
        }
    }

    pub fn gun(&self) -> Option<&Gun> {
        self.guns[self.cur].as_ref()
    }

    pub fn sliding(&self) -> bool {
        self.slide_t > 0.0
    }

    pub fn healing(&self) -> bool {
        self.heal_t > 0.0
    }

    /// Returns true if the fighter died from this hit.
    pub fn take_damage(&mut self, dmg: f32) -> bool {
        if !self.alive {
            return false;
        }
        let mut d = dmg;
        if self.armor > 0.0 {
            let a = (dmg * 0.5).min(self.armor);
            self.armor -= a;
            d -= a;
        }
        self.hp -= d;
        self.hit_flash = 0.15;
        self.heal_t = 0.0;
        if self.hp <= 0.0 {
            self.hp = 0.0;
            self.alive = false;
            return true;
        }
        false
    }

    /// Try to pick up a weapon. Returns true if it was taken.
    pub fn pick_weapon(&mut self, k: WKind) -> bool {
        if self.guns[1].is_none() && self.guns[0].as_ref().map(|g| g.kind) != Some(k) {
            let slot = if self.guns[0].is_none() { 0 } else { 1 };
            self.guns[slot] = Some(Gun::new(k));
            return true;
        }
        let sc = k.spec().score;
        let mut worst = 0;
        let mut worst_sc = i32::MAX;
        for i in 0..2 {
            if let Some(g) = &self.guns[i] {
                if g.kind == k {
                    return false;
                }
                let s = g.kind.spec().score;
                if s < worst_sc {
                    worst_sc = s;
                    worst = i;
                }
            }
        }
        if sc > worst_sc {
            self.guns[worst] = Some(Gun::new(k));
            return true;
        }
        false
    }

    pub fn update(&mut self, it: &Intent, dt: f32, w: &World, bullets: &mut Vec<Bullet>, sounds: &mut Vec<Sound>) {
        if !self.alive {
            return;
        }
        self.flash = (self.flash - dt).max(0.0);
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        self.bloom = (self.bloom - 2.2 * dt).max(0.0);
        self.slide_cd = (self.slide_cd - dt).max(0.0);
        self.aim = it.aim;

        if let Some(s) = it.swap {
            if s < 2 && self.guns[s].is_some() && s != self.cur {
                self.cur = s;
                self.heal_t = 0.0;
            }
        }

        // Healing channel
        if it.heal && self.heal_t <= 0.0 && self.medkits > 0 && self.hp < 100.0 {
            self.heal_t = 3.0;
        }
        let mut speed_mul = 1.0;
        if self.heal_t > 0.0 {
            speed_mul = 0.5;
            self.heal_t -= dt;
            if self.heal_t <= 0.0 {
                self.heal_t = 0.0;
                self.hp = (self.hp + 50.0).min(100.0);
                self.medkits -= 1;
            }
        }

        // Movement
        let want = if it.mv.length() > 1.0 { it.mv.normalize() } else { it.mv };
        let mut speed = WALK * speed_mul;
        if it.sprint && want.length() > 0.1 && !it.fire && self.heal_t <= 0.0 {
            speed = SPRINT;
        }
        if it.slide && self.slide_t <= 0.0 && self.slide_cd <= 0.0 && self.vel.length() > 150.0 {
            self.slide_t = SLIDE_TIME;
            self.slide_cd = 1.3;
            self.slide_dir = self.vel.normalize();
            self.heal_t = 0.0;
        }
        if self.slide_t > 0.0 {
            self.slide_t -= dt;
            let k = (self.slide_t / SLIDE_TIME).max(0.0);
            self.slide_dir = (self.slide_dir + want * dt * 2.0).normalize_or_zero();
            self.vel = self.slide_dir * (110.0 + 300.0 * k * k);
        } else {
            let target = want * speed;
            let a = 1.0 - (-13.0 * dt).exp();
            self.vel += (target - self.vel) * a;
        }
        self.step += self.vel.length() * dt;

        let mut p = self.pos;
        p.x += self.vel.x * dt;
        p = w.resolve(p, RADIUS);
        p.y += self.vel.y * dt;
        p = w.resolve(p, RADIUS);
        p.x = p.x.clamp(RADIUS, MAP - RADIUS);
        p.y = p.y.clamp(RADIUS, MAP - RADIUS);
        self.pos = p;

        // Weapon
        let (pos, aim, id, vel_len, bloom, sliding) = (self.pos, self.aim, self.id, self.vel.length(), self.bloom, self.sliding());
        let mut shot: Option<(WKind, f32)> = None;
        if let Some(g) = self.guns[self.cur].as_mut() {
            g.update(dt);
            if it.reload || (g.ammo == 0 && !g.reloading()) {
                g.start_reload();
            }
            if it.fire && g.ready() && self.heal_t <= 0.0 {
                if g.fire() {
                    shot = Some((g.kind, bloom));
                }
            }
        }
        if let Some((kind, bl)) = shot {
            let s = kind.spec();
            for _ in 0..s.pellets {
                let spread = s.spread + bl * 0.012 + (vel_len / SPRINT) * 0.035 + if sliding { 0.02 } else { 0.0 };
                let a = aim + rnd(-1.0, 1.0) * spread;
                bullets.push(Bullet {
                    pos: pos + dir(a) * (RADIUS + 6.0),
                    vel: dir(a) * s.speed,
                    dmg: s.dmg,
                    owner: id,
                    life: s.range / s.speed,
                    color: s.color,
                });
            }
            self.bloom = (self.bloom + s.bloom).min(3.0);
            self.flash = 0.06;
            sounds.push(Sound { pos, age: 0.0, owner: id });
        }
    }
}
