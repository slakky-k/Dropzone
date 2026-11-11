use crate::fighter::*;
use crate::game::*;
use crate::util::*;
use crate::weapons::*;
use crate::world::*;
use macroquad::prelude::*;

fn text_c(s: &str, x: f32, y: f32, size: f32, color: Color) {
    let d = measure_text(s, None, size as u16, 1.0);
    draw_text(s, x - d.width / 2.0, y, size, color);
}

fn rect_s(r: &Rect, v: Vec2, c: Color) {
    draw_rectangle(r.x + v.x, r.y + v.y, r.w, r.h, c);
}

pub fn draw(g: &Game) {
    let (sw, sh) = (screen_width(), screen_height());
    clear_background(Color::from_rgba(14, 18, 14, 255));
    let v = g.view();
    let vis = Rect::new(-v.x - 60.0, -v.y - 60.0, sw + 120.0, sh + 120.0);

    // Ground + grid
    draw_rectangle(v.x, v.y, MAP, MAP, Color::from_rgba(52, 66, 48, 255));
    let mut gx = (vis.x / 200.0).floor() * 200.0;
    while gx < vis.x + vis.w {
        if gx >= 0.0 && gx <= MAP {
            draw_line(gx + v.x, v.y.max(0.0), gx + v.x, v.y + MAP, 1.0, Color::new(1.0, 1.0, 1.0, 0.04));
        }
        gx += 200.0;
    }
    let mut gy = (vis.y / 200.0).floor() * 200.0;
    while gy < vis.y + vis.h {
        if gy >= 0.0 && gy <= MAP {
            draw_line(v.x, gy + v.y, v.x + MAP, gy + v.y, 1.0, Color::new(1.0, 1.0, 1.0, 0.04));
        }
        gy += 200.0;
    }

    // Floors, loot, walls
    for b in &g.w.buildings {
        if b.overlaps(&vis) {
            rect_s(b, v, Color::from_rgba(92, 88, 80, 255));
        }
    }
    for l in &g.w.loot {
        if l.taken || !vis.contains(l.pos) {
            continue;
        }
        let p = l.pos + v;
        let pulse = 1.0 + (g.t * 4.0 + l.pos.x).sin() * 0.12;
        match l.kind {
            LootKind::Weapon(k) => {
                let s = k.spec();
                draw_circle(p.x, p.y, 11.0 * pulse, Color::new(0.0, 0.0, 0.0, 0.35));
                draw_rectangle(p.x - 9.0, p.y - 4.0, 18.0, 8.0, s.color);
                draw_circle_lines(p.x, p.y, 12.0 * pulse, 2.0, s.color);
            }
            LootKind::Armor => {
                draw_poly(p.x, p.y, 6, 10.0 * pulse, 0.0, Color::from_rgba(70, 150, 255, 255));
                draw_poly_lines(p.x, p.y, 6, 12.0 * pulse, 0.0, 2.0, Color::from_rgba(200, 230, 255, 255));
            }
            LootKind::Medkit => {
                draw_rectangle(p.x - 9.0, p.y - 9.0, 18.0, 18.0, WHITE);
                draw_rectangle(p.x - 2.0, p.y - 7.0, 4.0, 14.0, Color::from_rgba(220, 40, 40, 255));
                draw_rectangle(p.x - 7.0, p.y - 2.0, 14.0, 4.0, Color::from_rgba(220, 40, 40, 255));
            }
        }
    }
    for r in &g.w.walls {
        if r.overlaps(&vis) {
            rect_s(r, v, Color::from_rgba(36, 34, 40, 255));
        }
    }
    for r in &g.w.rocks {
        if r.overlaps(&vis) {
            rect_s(r, v, Color::from_rgba(104, 108, 112, 255));
        }
    }

    // Fighters (enemies only drawn when in line of sight, or very close)
    let me = &g.fs[0];
    for f in g.fs.iter().skip(1) {
        if !f.alive || !vis.contains(f.pos) {
            continue;
        }
        if me.alive && f.pos.distance(me.pos) > 90.0 && !g.w.los(me.pos, f.pos) {
            continue;
        }
        draw_fighter(g, f, v);
    }
    if me.alive {
        draw_fighter(g, me, v);
        if me.reload_bar().is_some() {
            let p = me.pos + v;
            let k = me.reload_bar().unwrap();
            draw_rectangle(p.x - 18.0, p.y + 22.0, 36.0, 5.0, Color::new(0.0, 0.0, 0.0, 0.5));
            draw_rectangle(p.x - 18.0, p.y + 22.0, 36.0 * k, 5.0, WHITE);
        }
    }

    // Bullets and sparks
    for b in &g.bullets {
        let p = b.pos + v;
        let tail = p - b.vel.normalize_or_zero() * 22.0;
        draw_line(tail.x, tail.y, p.x, p.y, 2.5, b.color);
    }
    for s in &g.sparks {
        let p = s.pos + v;
        let mut c = s.color;
        c.a = (s.life / s.max).clamp(0.0, 1.0);
        draw_circle(p.x, p.y, 2.5, c);
    }

    // Storm
    let z = &g.w.zone;
    let c = z.c + v;
    for k in 0..12 {
        draw_poly_lines(c.x, c.y, 120, z.r + 100.0 + k as f32 * 200.0, 0.0, 200.0, Color::new(0.75, 0.1, 0.9, 0.10));
    }
    draw_poly_lines(c.x, c.y, 160, z.r, 0.0, 5.0, Color::new(0.9, 0.3, 1.0, 0.9));
    let c1 = z.c1 + v;
    if !z.done {
        draw_poly_lines(c1.x, c1.y, 160, z.r1, 0.0, 3.0, Color::new(1.0, 1.0, 1.0, 0.6));
    }

    draw_hud(g, v);
    g.ctl.draw();
}

trait ReloadBar {
    fn reload_bar(&self) -> Option<f32>;
}
impl ReloadBar for Fighter {
    fn reload_bar(&self) -> Option<f32> {
        let g = self.gun()?;
        if g.reloading() {
            Some(1.0 - g.reload_t / g.kind.spec().reload)
        } else {
            None
        }
    }
}

fn draw_fighter(g: &Game, f: &Fighter, v: Vec2) {
    let p = f.pos + v;
    let is_me = f.id == 0;
    let base = if is_me {
        Color::from_rgba(255, 138, 31, 255)
    } else {
        hsl_to_rgb((f.id as f32 * 0.137) % 1.0, 0.45, 0.42)
    };
    let r = if f.sliding() { RADIUS * 0.85 } else { RADIUS };
    draw_circle(p.x + 3.0, p.y + 4.0, RADIUS, Color::new(0.0, 0.0, 0.0, 0.25));
    if f.sliding() {
        let d = f.vel.normalize_or_zero();
        for k in 1..5 {
            let q = p - d * (k as f32 * 11.0);
            draw_circle(q.x, q.y, r * (1.0 - k as f32 * 0.18), Color::new(1.0, 1.0, 1.0, 0.12));
        }
    }
    let a = dir(f.aim);
    draw_circle(p.x, p.y, r, base);
    draw_circle_lines(p.x, p.y, r, 2.0, Color::new(0.0, 0.0, 0.0, 0.5));
    if let Some(gun) = f.gun() {
        let s = gun.kind.spec();
        let tip = p + a * (r + 17.0);
        draw_line(p.x + a.x * 5.0, p.y + a.y * 5.0, tip.x, tip.y, 5.0, s.color);
        if f.flash > 0.0 {
            draw_circle(tip.x + a.x * 4.0, tip.y + a.y * 4.0, 9.0, Color::from_rgba(255, 230, 120, 230));
        }
    }
    // Head: beanie + visor (original operator look)
    let head = p + a * 3.0;
    draw_circle(head.x, head.y, 8.0, if is_me { Color::from_rgba(30, 170, 170, 255) } else { Color::from_rgba(40, 40, 44, 255) });
    let vz = p + a * 8.0;
    draw_circle(vz.x, vz.y, 3.4, Color::from_rgba(20, 20, 24, 255));
    if f.hit_flash > 0.0 {
        draw_circle(p.x, p.y, r, Color::new(1.0, 1.0, 1.0, 0.5));
    }
    if f.healing() {
        draw_circle_lines(p.x, p.y, r + 7.0, 3.0, Color::from_rgba(90, 255, 120, 230));
    }
    if !is_me {
        let me = &g.fs[0];
        if me.pos.distance(f.pos) < 520.0 {
            text_c(&f.name, p.x, p.y - 26.0, 16.0, Color::new(1.0, 1.0, 1.0, 0.8));
            if f.hp < 100.0 {
                draw_rectangle(p.x - 16.0, p.y - 22.0, 32.0, 3.0, Color::new(0.0, 0.0, 0.0, 0.6));
                draw_rectangle(p.x - 16.0, p.y - 22.0, 32.0 * (f.hp / 100.0), 3.0, Color::from_rgba(230, 70, 60, 255));
            }
        }
    } else {
        draw_poly(p.x, p.y - 28.0, 3, 6.0, 90.0, Color::from_rgba(255, 138, 31, 255));
    }
}

fn draw_hud(g: &Game, v: Vec2) {
    let (sw, sh) = (screen_width(), screen_height());
    let fs = (sh / 34.0).clamp(16.0, 28.0);
    let me = &g.fs[0];
    let z = &g.w.zone;

    // Hurt vignette
    if g.hurt_t > 0.0 {
        let a = (g.hurt_t / 0.4).min(1.0) * 0.35;
        let c = Color::new(0.9, 0.0, 0.0, a);
        let t = sh * 0.08;
        draw_rectangle(0.0, 0.0, sw, t, c);
        draw_rectangle(0.0, sh - t, sw, t, c);
        draw_rectangle(0.0, 0.0, t, sh, c);
        draw_rectangle(sw - t, 0.0, t, sh, c);
    }

    // Top centre: alive / kills / zone
    text_c(&format!("ALIVE {}", g.alive_count()), sw / 2.0 - 90.0, fs + 6.0, fs, WHITE);
    text_c(&format!("KILLS {}", me.kills), sw / 2.0 + 90.0, fs + 6.0, fs, WHITE);
    let cd = z.countdown().max(0.0) as i32;
    let label = if z.done {
        "FINAL CIRCLE".to_string()
    } else if z.shrinking {
        format!("STORM CLOSING {}:{:02}", cd / 60, cd % 60)
    } else {
        format!("NEXT CIRCLE {}:{:02}", cd / 60, cd % 60)
    };
    text_c(&label, sw / 2.0, fs * 2.4 + 6.0, fs * 0.85, Color::from_rgba(235, 150, 255, 255));
    if me.alive && !z.inside(me.pos) {
        text_c(&format!("IN THE STORM  -{:.0}/s", z.dps()), sw / 2.0, fs * 4.0, fs * 1.1, Color::from_rgba(255, 90, 90, 255));
    }

    // Kill feed
    for (i, (s, t)) in g.feed.iter().enumerate() {
        let d = measure_text(s, None, (fs * 0.8) as u16, 1.0);
        draw_text(s, sw - d.width - 16.0, fs + 6.0 + i as f32 * fs, fs * 0.8, Color::new(1.0, 1.0, 1.0, (t / 1.0).min(1.0)));
    }

    // Health / armor / weapon
    let bw = (sw * 0.22).clamp(150.0, 260.0);
    let bx = sw / 2.0 - bw / 2.0 - sw * 0.06;
    let by = sh - fs * 3.2;
    draw_rectangle(bx, by, bw, 10.0, Color::new(0.0, 0.0, 0.0, 0.5));
    draw_rectangle(bx, by, bw * (me.hp / 100.0), 10.0, Color::from_rgba(90, 220, 110, 255));
    draw_rectangle(bx, by - 14.0, bw, 6.0, Color::new(0.0, 0.0, 0.0, 0.5));
    draw_rectangle(bx, by - 14.0, bw * (me.armor / 100.0), 6.0, Color::from_rgba(80, 160, 255, 255));
    draw_text(&format!("HP {:.0}", me.hp), bx, by + fs, fs * 0.8, WHITE);
    draw_text(&format!("MEDKITS {}", me.medkits), bx + bw * 0.5, by + fs, fs * 0.8, WHITE);
    for slot in 0..2 {
        if let Some(gun) = &me.guns[slot] {
            let s = gun.kind.spec();
            let sel = slot == me.cur;
            let txt = format!("{}{}  {}/{}", if sel { "> " } else { "  " }, s.name, gun.ammo, s.mag);
            let col = if sel { s.color } else { Color::new(1.0, 1.0, 1.0, 0.45) };
            draw_text(&txt, bx, by - 26.0 - (1 - slot) as f32 * fs, fs * 0.9, col);
        }
    }

    // Crosshair + hit marker
    if me.alive {
        let ps = me.pos + v;
        let ch = ps + dir(me.aim) * 120.0;
        let spread = 6.0 + me.bloom * 4.0;
        let cc = Color::new(1.0, 1.0, 1.0, 0.6);
        draw_line(ch.x - spread - 6.0, ch.y, ch.x - spread, ch.y, 2.0, cc);
        draw_line(ch.x + spread, ch.y, ch.x + spread + 6.0, ch.y, 2.0, cc);
        draw_line(ch.x, ch.y - spread - 6.0, ch.x, ch.y - spread, 2.0, cc);
        draw_line(ch.x, ch.y + spread, ch.x, ch.y + spread + 6.0, 2.0, cc);
        if g.hitmark > 0.0 {
            let c = if g.hit_kill { Color::from_rgba(255, 60, 60, 255) } else { WHITE };
            let k = 10.0;
            draw_line(ch.x - k, ch.y - k, ch.x - 4.0, ch.y - 4.0, 3.0, c);
            draw_line(ch.x + k, ch.y - k, ch.x + 4.0, ch.y - 4.0, 3.0, c);
            draw_line(ch.x - k, ch.y + k, ch.x - 4.0, ch.y + 4.0, 3.0, c);
            draw_line(ch.x + k, ch.y + k, ch.x + 4.0, ch.y + 4.0, 3.0, c);
        }
    }
    if g.notice.1 > 0.0 {
        text_c(&g.notice.0, sw / 2.0, sh * 0.72, fs, Color::new(1.0, 1.0, 1.0, g.notice.1.min(1.0)));
    }

    draw_minimap(g, sh);

    // End screens
    if let Some(won) = g.over {
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.6));
        if won {
            text_c("VICTORY", sw / 2.0, sh * 0.4, fs * 3.0, Color::from_rgba(255, 210, 80, 255));
        } else {
            text_c("ELIMINATED", sw / 2.0, sh * 0.4, fs * 3.0, Color::from_rgba(255, 90, 80, 255));
        }
        text_c(&format!("Placement #{}   Kills {}", g.place, me.kills), sw / 2.0, sh * 0.52, fs * 1.3, WHITE);
        text_c("Tap or press ENTER to redeploy", sw / 2.0, sh * 0.64, fs, Color::new(1.0, 1.0, 1.0, 0.8));
    }
}

fn draw_minimap(g: &Game, sh: f32) {
    let ms = (sh * 0.28).clamp(110.0, 170.0);
    let (x0, y0) = (12.0, 12.0);
    let k = ms / MAP;
    draw_rectangle(x0, y0, ms, ms, Color::new(0.0, 0.0, 0.0, 0.55));
    for b in &g.w.buildings {
        draw_rectangle(x0 + b.x * k, y0 + b.y * k, (b.w * k).max(2.0), (b.h * k).max(2.0), Color::new(0.7, 0.7, 0.7, 0.6));
    }
    let clampp = |p: Vec2| vec2(p.x.clamp(0.0, ms), p.y.clamp(0.0, ms));
    let ring = |c: Vec2, r: f32, col: Color| {
        let n = 72;
        let mut prev: Option<Vec2> = None;
        for i in 0..=n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let p = clampp((c + dir(a) * r) * k);
            if let Some(q) = prev {
                if (p - q).length() < ms {
                    draw_line(x0 + q.x, y0 + q.y, x0 + p.x, y0 + p.y, 1.5, col);
                }
            }
            prev = Some(p);
        }
    };
    ring(g.w.zone.c, g.w.zone.r, Color::new(0.9, 0.3, 1.0, 0.9));
    if !g.w.zone.done {
        ring(g.w.zone.c1, g.w.zone.r1, Color::new(1.0, 1.0, 1.0, 0.8));
    }
    let me = &g.fs[0];
    let p = clampp(me.pos * k);
    draw_circle(x0 + p.x, y0 + p.y, 3.5, Color::from_rgba(255, 138, 31, 255));
    let tip = p + dir(me.aim) * 9.0;
    draw_line(x0 + p.x, y0 + p.y, x0 + tip.x, y0 + tip.y, 2.0, WHITE);
    draw_rectangle_lines(x0, y0, ms, ms, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
}
