use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WKind {
    Pistol,
    Smg,
    Ar,
    Shotgun,
    Sniper,
}

pub struct Spec {
    pub name: &'static str,
    pub dmg: f32,
    pub rpm: f32,
    pub mag: i32,
    pub reload: f32,
    pub spread: f32,
    pub bloom: f32,
    pub speed: f32,
    pub range: f32,
    pub pellets: i32,
    pub color: Color,
    pub score: i32,
    pub pref_range: f32,
    pub auto: bool,
}

impl WKind {
    pub fn spec(self) -> Spec {
        match self {
            WKind::Pistol => Spec {
                name: "P-9 Sidearm", dmg: 18.0, rpm: 330.0, mag: 12, reload: 1.1,
                spread: 0.02, bloom: 0.5, speed: 1100.0, range: 600.0, pellets: 1,
                color: Color::new(0.8, 0.8, 0.8, 1.0), score: 1, pref_range: 250.0, auto: false,
            },
            WKind::Smg => Spec {
                name: "VX-5 SMG", dmg: 11.0, rpm: 850.0, mag: 30, reload: 1.5,
                spread: 0.035, bloom: 0.35, speed: 1000.0, range: 520.0, pellets: 1,
                color: Color::new(0.4, 0.9, 1.0, 1.0), score: 3, pref_range: 260.0, auto: true,
            },
            WKind::Ar => Spec {
                name: "KR-47 Rifle", dmg: 20.0, rpm: 600.0, mag: 30, reload: 2.0,
                spread: 0.02, bloom: 0.45, speed: 1300.0, range: 900.0, pellets: 1,
                color: Color::new(1.0, 0.8, 0.3, 1.0), score: 4, pref_range: 380.0, auto: true,
            },
            WKind::Shotgun => Spec {
                name: "Breaker-12", dmg: 12.0, rpm: 75.0, mag: 6, reload: 2.4,
                spread: 0.11, bloom: 0.2, speed: 900.0, range: 320.0, pellets: 8,
                color: Color::new(1.0, 0.45, 0.35, 1.0), score: 3, pref_range: 170.0, auto: false,
            },
            WKind::Sniper => Spec {
                name: "Longbow SR", dmg: 95.0, rpm: 48.0, mag: 5, reload: 2.8,
                spread: 0.002, bloom: 0.0, speed: 2400.0, range: 1800.0, pellets: 1,
                color: Color::new(0.7, 1.0, 0.5, 1.0), score: 3, pref_range: 650.0, auto: false,
            },
        }
    }
}

#[derive(Clone)]
pub struct Gun {
    pub kind: WKind,
    pub ammo: i32,
    pub cd: f32,
    pub reload_t: f32,
}

impl Gun {
    pub fn new(kind: WKind) -> Self {
        Gun { kind, ammo: kind.spec().mag, cd: 0.0, reload_t: 0.0 }
    }
    pub fn reloading(&self) -> bool {
        self.reload_t > 0.0
    }
    pub fn ready(&self) -> bool {
        self.cd <= 0.0 && self.reload_t <= 0.0 && self.ammo > 0
    }
    pub fn can_reload(&self) -> bool {
        !self.reloading() && self.ammo < self.kind.spec().mag
    }
    pub fn start_reload(&mut self) {
        if self.can_reload() {
            self.reload_t = self.kind.spec().reload;
        }
    }
    pub fn update(&mut self, dt: f32) {
        if self.cd > 0.0 {
            self.cd -= dt;
        }
        if self.reload_t > 0.0 {
            self.reload_t -= dt;
            if self.reload_t <= 0.0 {
                self.reload_t = 0.0;
                self.ammo = self.kind.spec().mag;
            }
        }
    }
    /// Consume a round. Returns true if a shot was taken.
    pub fn fire(&mut self) -> bool {
        if !self.ready() {
            return false;
        }
        self.ammo -= 1;
        self.cd = 60.0 / self.kind.spec().rpm;
        true
    }
}
