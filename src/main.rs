mod bot;
mod fighter;
mod game;
mod input;
mod render;
mod util;
mod weapons;
mod world;

use game::Game;
use input::Controls;
use macroquad::prelude::*;

fn conf() -> Conf {
    Conf {
        window_title: "DROPZONE".to_string(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

fn new_game(ctl: Controls) -> Game {
    macroquad::rand::srand((macroquad::miniquad::date::now() * 1000.0) as u64);
    Game::new(ctl)
}

fn tapped() -> bool {
    is_key_pressed(KeyCode::Enter)
        || is_mouse_button_pressed(MouseButton::Left)
        || touches().iter().any(|t| t.phase == TouchPhase::Started)
}

#[macroquad::main(conf)]
async fn main() {
    // Title screen
    loop {
        clear_background(Color::from_rgba(14, 18, 14, 255));
        let (sw, sh) = (screen_width(), screen_height());
        let t = get_time() as f32;
        let big = sh / 8.0;
        let d = measure_text("DROPZONE", None, big as u16, 1.0);
        draw_text("DROPZONE", sw / 2.0 - d.width / 2.0, sh * 0.38, big, Color::from_rgba(255, 138, 31, 255));
        let sub = "Operator: RIVET   |   24 players   |   last one standing";
        let d = measure_text(sub, None, (sh / 26.0) as u16, 1.0);
        draw_text(sub, sw / 2.0 - d.width / 2.0, sh * 0.48, sh / 26.0, WHITE);
        let hint = "Tap or press ENTER to deploy";
        let d = measure_text(hint, None, (sh / 24.0) as u16, 1.0);
        draw_text(hint, sw / 2.0 - d.width / 2.0, sh * 0.62, sh / 24.0, Color::new(1.0, 1.0, 1.0, 0.5 + 0.5 * (t * 3.0).sin().abs()));
        let help = "PC: WASD move, mouse aim/fire, Shift sprint, C slide, R reload, H heal, 1/2/Q swap";
        let d = measure_text(help, None, (sh / 36.0) as u16, 1.0);
        draw_text(help, sw / 2.0 - d.width / 2.0, sh * 0.78, sh / 36.0, Color::new(1.0, 1.0, 1.0, 0.55));
        let help2 = "Touch: left thumb moves (push fully to sprint), right thumb aims and fires";
        let d = measure_text(help2, None, (sh / 36.0) as u16, 1.0);
        draw_text(help2, sw / 2.0 - d.width / 2.0, sh * 0.83, sh / 36.0, Color::new(1.0, 1.0, 1.0, 0.55));
        if tapped() {
            break;
        }
        next_frame().await;
    }

    let mut game = new_game(Controls::new());
    loop {
        if game.over.is_some() && game.t > 0.5 && tapped() {
            let ctl = std::mem::replace(&mut game.ctl, Controls::new());
            game = new_game(ctl);
        }
        let dt = get_frame_time();
        game.update(dt);
        render::draw(&game);
        next_frame().await;
    }
}
