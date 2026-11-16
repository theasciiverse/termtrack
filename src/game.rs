use crate::input::{Input, Key};
use crate::loader::{self};
use crate::renderer::{self, *};
use crate::GW;
use crate::LevelMap;
use crate::{camera::Camera, mat::*};
use crate::{screens, GH};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Game {
    pub renderer: Screen,
    pub camera: Camera,
}

const SPEED: f64 = 30.;
const MAX_FALL_SPEED: f64 = 130.;
const JUMP_SPEED: f64 = 53.;
const ROTATION_SPEED: f64 = 2.5;
const GRAVITY: f64 = 125.;
const PLAYER_COLLIDER: ((f64, f64, f64), (f64, f64, f64)) = ((-0.2, 4.5, -0.2), (0.2, -1., 0.2));

impl Game {
    pub fn run(
        &mut self,
        map: loader::LevelMap,
        input: &Input,
    ) -> Result<f64, &'static str> {
        // load map files
        // generate map meshes

        let LevelMap {
            mesh,
            colliders,
            start_pos: start,
            map_string,
            level_name,
            mut enemies,
        } = map;

        self.camera.pos = Vec3 {
            x: start.0,
            y: start.1,
            z: start.2,
        };

        let mut level_timer = 0.;

        // timer for fps
        let mut time = Instant::now();

        // for the session log's snapshots
        let mut last_snapshot = Instant::now();

        let floors = renderer::map_as_vec_of_floors(&map_string).len();

        let mut started = false;

        loop {
            // reset timer for dt
            let dt = time.elapsed().as_secs_f64();
            time = Instant::now();

            // text for infobar
            let fps_text = format!("fps: {:.2?} ", 1. / (dt));
            let timer_text = format!("time: {:.1?} ", level_timer);
            let floor_text = format!(
                "floor: {}/{}",
                (-self.camera.pos.y.div_euclid(GH) + 1.).clamp(0., floors as f64) as usize,
                floors
            );

            // limit dt for low framerate
            let dt = dt.min(0.2);
            //update timer if started moving
            if started {
                level_timer += dt;
            }

            //update enemies
            let mut render_mesh = mesh.clone();
            let mut cols = colliders.clone();
            for enemy in enemies.iter_mut() {
                enemy.update(dt, self.camera.pos, &colliders);
                cols.push(enemy.get_collider());
                render_mesh = render_mesh + enemy.get_mesh();
            }

            // render vertices in parallel thread.
            let cam = self.camera.clone();
            let renderer = self.renderer.clone();
            let render_thread =
                thread::spawn(move || renderer.render_pruned_mt(&cam, &render_mesh));

            // one-shot keys
            while let Some(key) = input.next_press() {
                match key {
                    Key::E | Key::Escape => {
                        let time1 = time.elapsed();
                        log::info!("state: exit asked");
                        if screens::exit(input) {
                            return Err("menu");
                        };
                        log::info!("state: back to the level");
                        input.clear_presses();
                        time = Instant::now().checked_sub(time1).unwrap();
                        break;
                    }
                    Key::Quit => screens::exit_app(),
                    Key::R => return Err("retry"),
                    Key::M => {
                        let time1 = time.elapsed();
                        log::info!("state: map");
                        self.renderer
                            .render_map(&map_string, self.camera.pos, loader::GW, loader::GH);
                        thread::sleep(Duration::from_millis(150));
                        input.clear_presses();
                        loop {
                            match input.wait_press() {
                                Key::M | Key::E | Key::Escape | Key::Enter => break,
                                Key::Quit => screens::exit_app(),
                                _ => (),
                            }
                        }
                        log::info!("state: back to the level");
                        input.clear_presses();
                        time = Instant::now().checked_sub(time1).unwrap();
                        break;
                    }
                    _ => (),
                }
            }

            let mut v = Vec3 {
                x: 0.,
                y: self.camera.vel.y,
                z: 0.,
            };

            // held keys
            {
                if input.held(Key::Left) {
                    self.camera.rotation.x -= ROTATION_SPEED * dt;
                }
                if input.held(Key::Right) {
                    self.camera.rotation.x += ROTATION_SPEED * dt;
                }
                if input.held(Key::Up) && self.camera.rotation.y < 1.5 {
                    self.camera.rotation.y += ROTATION_SPEED * dt;
                }
                if input.held(Key::Down) && self.camera.rotation.y > -1.5 {
                    self.camera.rotation.y -= ROTATION_SPEED * dt;
                }

                if input.held(Key::W) {
                    v = v + Vec3 {
                        x: 0.,
                        y: 0.,
                        z: SPEED,
                    }
                    .rotate_y(self.camera.rotation.x);
                    started = true;
                }
                if input.held(Key::A) {
                    v = v + Vec3 {
                        x: -SPEED,
                        y: 0.,
                        z: 0.,
                    }
                    .rotate_y(self.camera.rotation.x);
                    started = true;
                }
                if input.held(Key::D) {
                    v = v + Vec3 {
                        x: SPEED,
                        y: 0.,
                        z: 0.,
                    }
                    .rotate_y(self.camera.rotation.x);
                    started = true;
                }
                if input.held(Key::S) {
                    v = v + Vec3 {
                        x: 0.,
                        y: 0.,
                        z: -SPEED,
                    }
                    .rotate_y(self.camera.rotation.x);
                    started = true;
                }
            } // add gravity
            self.camera.vel = v;
            self.camera.vel.y += GRAVITY * dt;
            self.camera.vel.y = self.camera.vel.y.min(MAX_FALL_SPEED);

            // collision
            let mut current_pc = BoxCollider::new(PLAYER_COLLIDER.0, PLAYER_COLLIDER.1, None);
            let mut grounded = false;
            if let Some(tag) = check_collision(
                &mut current_pc,
                &mut self.camera.pos,
                &mut self.camera.vel,
                dt,
                &cols,
                &mut grounded,
            ) {
                return match tag {
                    "goal" => Ok(level_timer),
                    "angry_pixel" => Err("angry_pixel"),
                    "spike" => Err("spike"),
                    t => panic!("unkown collider-tag: {t}"),
                };
            };
            self.camera.update_pos(dt);

            //print to when rendering is finished screen
            if let Ok(buffer) = render_thread.join() {
                self.renderer.flush(
                    &buffer,
                    false,
                    &format!("{}{}{}", &fps_text, &timer_text, &floor_text),
                );
            }

            // jump
            if grounded && input.held(Key::Space) {
                self.camera.vel.y = -JUMP_SPEED;
                log::debug!("jump");
                started = true;
            }

            log::trace!(
                "frame: pos {:.3} {:.3} {:.3} turn {:.4} {:.4} grounded {} time {:.3}",
                self.camera.pos.x, self.camera.pos.y, self.camera.pos.z,
                self.camera.rotation.x, self.camera.rotation.y,
                grounded, level_timer,
            );
            if last_snapshot.elapsed() >= Duration::from_millis(500) {
                last_snapshot = Instant::now();
                log::debug!(
                    "snapshot: level {} pos ({:.2}, {:.2}, {:.2}) turn ({:.2}, {:.2}) {}time {:.2} grounded {} held {:?} fps {:.0}",
                    level_name,
                    self.camera.pos.x, self.camera.pos.y, self.camera.pos.z,
                    self.camera.rotation.x, self.camera.rotation.y,
                    floor_text.clone() + " ",
                    level_timer,
                    grounded,
                    input.held_keys(),
                    1. / dt.max(1e-6),
                );
            }

            if self.camera.pos.y > GW * (floors + 10) as f64 {
                return Err("void");
            }
        }
    }
}
