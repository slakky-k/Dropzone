use crate::fighter::Intent;
use crate::util::*;
use macroquad::prelude::*;

const STICK_R: f32 = 80.0;
const FIRE_RING: f32 = 55.0;

#[derive(Clone, Copy)]
pub struct Btn {
    pub c: Vec2,
    pub r: f32,
    pub label: &'static str,
}

pub fn buttons() -> [Btn; 4] {
    let (w, h) = (screen_width(), screen_height());
    let u = (h * 0.09).clamp(30.0, 56.0);
    [
        Btn { c: vec2(w - u * 1.6, h - u * 5.2), r: u, label: "RELOAD" },
        Btn { c: vec2(w - u * 3.6, h - u * 3.4), r: u, label: "SLIDE" },
        Btn { c: vec2(w - u * 1.6, h - u * 7.4), r: u * 0.85, label: "SWAP" },
        Btn { c: vec2(w - u * 3.6, h - u * 5.9), r: u * 0.85, label: "HEAL" },
    ]
}

pub struct Controls {
    pub touch_mode: bool,
    move_id: Option<u64>,
    move_origin: Vec2,
    move_pos: Vec2,
    aim_id: Option<u64>,
    aim_origin: Vec2,
    aim_pos: Vec2,
    aim_angle: f32,
}

impl Controls {
    pub fn new() -> Self {
        Controls {
            touch_mode: false, move_id: None, move_origin: Vec2::ZERO, move_pos: Vec2::ZERO,
            aim_id: None, aim_origin: Vec2::ZERO, aim_pos: Vec2::ZERO, aim_angle: 0.0,
        }
    }

    /// `player_screen` is where the player is drawn (for mouse aiming).
    /// `assist` is the angle to the best auto-aim candidate, if any.
    pub fn read(&mut self, player_screen: Vec2, assist: Option<f32>) -> Intent {
        let mut it = Intent::default();
        let sw = screen_width();
        let touches = touches();
        let btns = buttons();
        let mut pressed = [false; 4];

        if !touches.is_empty() {
            self.touch_mode = true;
        }
        for t in &touches {
            let p = t.position;
            match t.phase {
                TouchPhase::Started => {
                    if let Some(bi) = btns.iter().position(|b| b.c.distance(p) < b.r * 1.2) {
                        pressed[bi] = true;
                    } else if p.x < sw * 0.5 && self.move_id.is_none() {
                        self.move_id = Some(t.id);
                        self.move_origin = p;
                        self.move_pos = p;
                    } else if p.x >= sw * 0.5 && self.aim_id.is_none() {
                        self.aim_id = Some(t.id);
                        self.aim_origin = p;
                        self.aim_pos = p;
                    }
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    if self.move_id == Some(t.id) {
                        self.move_id = None;
                    }
                    if self.aim_id == Some(t.id) {
                        self.aim_id = None;
                    }
                }
                _ => {
                    if self.move_id == Some(t.id) {
                        self.move_pos = p;
                    }
                    if self.aim_id == Some(t.id) {
                        self.aim_pos = p;
                    }
                }
            }
        }

        if self.touch_mode {
            if self.move_id.is_some() {
                let v = (self.move_pos - self.move_origin) / STICK_R;
                it.mv = if v.length() > 1.0 { v.normalize() } else { v };
                if it.mv.length() < 0.12 {
                    it.mv = Vec2::ZERO;
                }
                it.sprint = it.mv.length() > 0.92;
            }
            if self.aim_id.is_some() {
                let v = self.aim_pos - self.aim_origin;
                if v.length() > 12.0 {
                    self.aim_angle = angle_of(v);
                }
                it.fire = v.length() > FIRE_RING;
                if it.fire {
                    if let Some(a) = assist {
                        let d = angle_diff(a, self.aim_angle);
                        if d.abs() < 0.22 {
                            self.aim_angle += d * 0.35;
                        }
                    }
                }
            }
            it.aim = self.aim_angle;
            it.reload = pressed[0];
            it.slide = pressed[1];
            if pressed[2] {
                it.swap = Some(2); // 2 = "toggle", resolved in main
            }
            it.heal = pressed[3];
        }

        // Keyboard + mouse (desktop testing)
        let mut kb = Vec2::ZERO;
        if is_key_down(KeyCode::W) { kb.y -= 1.0; }
        if is_key_down(KeyCode::S) { kb.y += 1.0; }
        if is_key_down(KeyCode::A) { kb.x -= 1.0; }
        if is_key_down(KeyCode::D) { kb.x += 1.0; }
        let mouse_used = is_mouse_button_down(MouseButton::Left) || is_mouse_button_pressed(MouseButton::Right);
        if kb != Vec2::ZERO || mouse_used {
            self.touch_mode = false;
        }
        if !self.touch_mode {
            it.mv = kb.normalize_or_zero();
            let (mx, my) = mouse_position();
            let to = vec2(mx, my) - player_screen;
            if to.length() > 4.0 {
                it.aim = angle_of(to);
                self.aim_angle = it.aim;
            } else {
                it.aim = self.aim_angle;
            }
            it.fire = is_mouse_button_down(MouseButton::Left);
            it.sprint = is_key_down(KeyCode::LeftShift);
            it.slide = is_key_pressed(KeyCode::C) || is_key_pressed(KeyCode::LeftControl);
            it.reload = is_key_pressed(KeyCode::R);
            it.heal = is_key_pressed(KeyCode::H);
            if is_key_pressed(KeyCode::Key1) {
                it.swap = Some(0);
            }
            if is_key_pressed(KeyCode::Key2) {
                it.swap = Some(1);
            }
            if is_key_pressed(KeyCode::Q) {
                it.swap = Some(2);
            }
            if assist.is_some() && it.fire {
                // light assist for mouse too is unnecessary; keep raw aim
            }
        }
        it
    }

    pub fn draw(&self) {
        if !self.touch_mode {
            return;
        }
        let ring = Color::new(1.0, 1.0, 1.0, 0.25);
        let fill = Color::new(1.0, 1.0, 1.0, 0.12);
        if self.move_id.is_some() {
            draw_circle(self.move_origin.x, self.move_origin.y, STICK_R, fill);
            draw_circle_lines(self.move_origin.x, self.move_origin.y, STICK_R, 2.0, ring);
            let v = (self.move_pos - self.move_origin).clamp_length_max(STICK_R);
            let p = self.move_origin + v;
            draw_circle(p.x, p.y, 30.0, Color::new(1.0, 1.0, 1.0, 0.45));
        } else {
            let (w, h) = (screen_width(), screen_height());
            draw_text("MOVE", w * 0.12, h * 0.8, 24.0, ring);
        }
        if self.aim_id.is_some() {
            draw_circle(self.aim_origin.x, self.aim_origin.y, STICK_R, fill);
            draw_circle_lines(self.aim_origin.x, self.aim_origin.y, FIRE_RING, 2.0, Color::new(1.0, 0.4, 0.3, 0.5));
            let v = (self.aim_pos - self.aim_origin).clamp_length_max(STICK_R);
            let p = self.aim_origin + v;
            let firing = v.length() > FIRE_RING;
            draw_circle(p.x, p.y, 30.0, if firing { Color::new(1.0, 0.35, 0.25, 0.7) } else { Color::new(1.0, 1.0, 1.0, 0.45) });
        } else {
            let (w, h) = (screen_width(), screen_height());
            draw_text("AIM / FIRE", w * 0.58, h * 0.8, 24.0, ring);
        }
        for b in buttons() {
            draw_circle(b.c.x, b.c.y, b.r, Color::new(1.0, 1.0, 1.0, 0.14));
            draw_circle_lines(b.c.x, b.c.y, b.r, 2.0, ring);
            let d = measure_text(b.label, None, 16, 1.0);
            draw_text(b.label, b.c.x - d.width / 2.0, b.c.y + 5.0, 16.0, Color::new(1.0, 1.0, 1.0, 0.8));
        }
    }
}
