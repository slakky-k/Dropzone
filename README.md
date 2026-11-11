# DROPZONE (Rust, macroquad)

Top-down mobile-style battle royale prototype. 24 fighters, shrinking storm, loot,
and bots with human-like behaviour. You play **RIVET**, an original operator.
Rename in `src/game.rs` (`PLAYER_NAME`).

## Run on PC
```
cargo run --release
```
WASD move, mouse aim + fire, Shift sprint, C slide, R reload, H heal, 1/2/Q swap weapon.

## Run on Android
```
cargo install cargo-quad-apk
rustup target add aarch64-linux-android
# Install Android SDK + NDK, set ANDROID_HOME and NDK_HOME
cargo quad-apk build --release
```
The APK ends up under `target/android-artifacts/`. Check the cargo-quad-apk docs if the
`[package.metadata.android]` keys in `Cargo.toml` have changed.

Touch controls: left thumb moves (push fully to sprint), right thumb aims, and
pushing past the red ring fires (with light aim assist). Buttons: RELOAD, SLIDE, SWAP, HEAL.

## What's in it
- `fighter.rs`  shared movement for player and bots: accel, sprint, slide, bloom, armor, heal channel
- `bot.rs`      bot brain: perception (FOV, line of sight, gunfire hearing), reaction time,
                target leading, aim noise and limited turn rate, burst fire, strafing, range
                keeping, cover + heal, flanking, third-party fights, early zone rotation, loot
                priorities, A* pathing with anti-stuck
- `world.rs`    procedural map (towns, buildings with doors, rocks), loot, storm phases, A*
- `game.rs`     bullets, damage, kills and drops, pickups, storm damage, camera
- `render.rs`   world, HUD, minimap  `input.rs` touch + keyboard/mouse

## Tuning
- Bot difficulty: `Brain::new(rnd(0.3, 0.95), ..)` in `game.rs` (higher = sharper)
- Bot count: `NBOTS` in `game.rs`
- Weapons: `src/weapons.rs`  Storm timings: top of `src/world.rs`

## Status
Written without being able to compile (the build environment had no network to fetch
crates), so expect a few compiler errors on first `cargo build`. Paste them back and they are quick to fix.

## Build the APK in the cloud (no PC)
1. Create a GitHub repo and upload this project (including `.github/workflows/android.yml`).
2. Open the **Actions** tab, run **Build Android APK** (it also runs on every push).
3. When it finishes, open the run and download the **dropzone-apk** artifact, unzip, install the .apk.
   If it fails, open the failed step and copy the error text.
