use std::sync::{Arc, Mutex};

use crate::{camera::Camera, mat::*};
use rayon::prelude::*;

pub const RENDER_DIST: f64 = 30.;

pub fn get_terminal_size() -> (usize, usize) {
    let (w, h) = crossterm::terminal::size().unwrap();
    (w as usize, h as usize)
}

/// Asciiverse: the size the 3D view is rendered at. Upstream capped the
/// view at 220x65 cells to keep the frame rate (every cell is a traced
/// ray) and left the rest of a bigger terminal black. Here the render is
/// scaled down evenly to fit that budget and then stretched over the whole
/// terminal by `flush`, so the view fills the screen at the same cost.
fn render_size(tw: usize, th: usize) -> (usize, usize) {
    let k = (220.0 / tw.max(1) as f64).min(65.0 / th.max(1) as f64).min(1.0);
    let w = ((tw as f64 * k).round() as usize).max(1);
    let h = ((th as f64 * k).round() as usize).max(1);
    (w, h)
}

#[derive(Clone)]
pub struct Screen {
    /// the rendered view, in cells
    pub w: usize,
    pub h: usize,
    /// the terminal it is shown on (th without the status line)
    pub tw: usize,
    pub th: usize,
}

impl Screen {
    pub fn new() -> Self {
        let (tw, th) = get_terminal_size();
        let mut stdout = std::io::stdout();
        crossterm::execute!(stdout, crossterm::cursor::Hide).unwrap();
        let th = th.saturating_sub(1).max(1);
        let (w, h) = render_size(tw, th);

        //clear screen
        print!("\x1b[2J\x1b[H\r");

        let buffer = vec![
            vec![
                Vec3 {
                    x: 0.,
                    y: 0.,
                    z: 0.
                };
                w
            ];
            h
        ];

        Screen { w, h, tw, th }
    }

    pub fn flush(&mut self, buffer: &[Vec<Vec3>], ascii: bool, extra: &str) {
        // Create new string buffer
        let mut pix_buffer = String::new();
        let mut color = Vec3 {
            x: 0.,
            y: 0.,
            z: 0.,
        };
        // Iterate over the terminal, each cell taking the rendered pixel
        // under it (the view is stretched to fill the terminal)
        let bh = buffer.len();
        let bw = buffer.first().map_or(0, |r| r.len());
        if bh == 0 || bw == 0 {
            return;
        }
        for ty in 0..self.th {
            let y = (ty * bh / self.th).min(bh - 1);
            for tx in 0..self.tw {
                let x = (tx * bw / self.tw).min(bw - 1);
                // Add ' ' (char if ascii is true) withrightcolor to string buffer
                let mut c = ' ';
                // set background
                let mut command = "48";
                let col = buffer[y][x];
                if ascii {
                    // set foreground
                    command = "38";
                    let chars = [
                        ' ', '.', '-', ':', '_', '~', '/', 'c', 'r', 'x', '*', '%', '#', '8', '@',
                    ];

                    let s = col.x + col.y + col.z;
                    let s = s * chars.len() as f64 / (255 * 3) as f64;
                    let s = (s as usize).clamp(0, chars.len() - 1);
                    c = chars[s];
                }
                if col != color {
                    pix_buffer += &format!(
                        "\x1b[{command};2;{};{};{}m{c}",
                        col.x as u8, col.y as u8, col.z as u8
                    );
                    color = col;
                } else {
                    pix_buffer += " ";
                }
            }
            pix_buffer += "\x1b[48;2;0;0;0m\r\n";
            // the background was just reset to black: forget the last colour,
            // or a row starting in it would draw its first cell black
            color = Vec3 {
                x: -1.,
                y: -1.,
                z: -1.,
            };
        }
        pix_buffer += &format!("{:<1$}", extra, self.tw);

        //Move cursor home and print string buffer
        print!("\x1b[H{}", pix_buffer);

        let (tw, th) = get_terminal_size();
        let th = th.saturating_sub(1).max(1);
        if self.tw != tw || self.th != th {
            print!("\x1b[2J\r");
            self.tw = tw;
            self.th = th;
            let (w, h) = render_size(tw, th);
            self.w = w;
            self.h = h;
        }
    }

    // Not used, left for benchmark against new version

    pub fn render_pruned_mt(&self, camera: &Camera, mesh: &Mesh) -> Vec<Vec<Vec3>> {
        let mut pruned_tris = Vec::with_capacity(mesh.tris().len());
        let forward = Vec3 {
            x: 0.,
            y: 0.,
            z: 1.,
        };
        let min_dim = self.w.min(self.h * 2) as f64 / 2.;
        let pixel_coords = Vec3 {
            x: -(self.w as f64 / 2.) / min_dim,
            y: -(self.h as f64 / 2.) / min_dim,
            z: camera.focus_length,
        };
        let angle = (forward.dot(pixel_coords) / pixel_coords.abs())
            .acos()
            .abs()
            + 5.;
        let forward = forward.rotate(camera.rotation);
        for tri in mesh.tris() {
            let mut valid = false;
            for p in [tri.v0, tri.v1, tri.v2] {
                // vector from pos to vertex
                let v = p - camera.pos;
                let v_abs = v.abs();
                if v_abs < RENDER_DIST {
                    valid = true;
                    break;
                }
                if (v.dot(forward) / v_abs).acos() <= angle {
                    valid = true;
                    break;
                }
            }
            if valid {
                pruned_tris.push(tri);
            }
        }
        self.render_mt(camera, &pruned_tris)
    }

    pub fn render_mt(&self, camera: &Camera, tris: &[Tri]) -> Vec<Vec<Vec3>> {
        let mut buffer = vec![
            vec![
                Vec3 {
                    x: 0.,
                    y: 0.,
                    z: 0.
                };
                self.w
            ];
            self.h
        ];

        buffer.par_iter_mut().enumerate().for_each(|(y, row)| {
            row.par_iter_mut().enumerate().for_each(|(x, pixel)| {
                let mut min_dist = f64::MAX;
                let mut color = Vec3 {
                    x: 0.,
                    y: 0.,
                    z: 0.,
                };
                let min_dim = self.w.min(self.h * 2) as f64 / 2.;
                let pixel_coords = Vec3 {
                    x: (x as f64 - self.w as f64 / 2.) / min_dim,
                    y: (y as f64 * 2. - self.h as f64 / 2.) / min_dim,
                    z: camera.focus_length,
                };
                let pixel_coords = pixel_coords.rotate(camera.rotation);
                let ray_dir = pixel_coords;
                let ray_o = camera.pos;
                let mut closet_idx = None;
                tris.iter().enumerate().for_each(|(idx, tri)| {
                    let (hit, distance) = tri.hit_mt(ray_o, ray_dir);
                    if hit && distance < min_dist {
                        min_dist = distance;

                        closet_idx = Some(idx);
                    }
                });
                if let Some(idx) = closet_idx {
                    let tri = tris[idx];
                    color = tri.color;
                    let n = tri.normal();
                    color = color
                        * (n.dot(ray_dir * (-1.)) / (ray_dir.abs() * n.abs()))
                            .abs()
                            .clamp(0.5, 1.);
                }
                color = color * (1. - min_dist / RENDER_DIST);
                *pixel = color;
            });
        });
        buffer
    }

    fn print_info(&self, camera: &Camera, extra: &str) {
        print!("\x1b[48;1;0m{:<1$}", extra, self.tw);
    }

    pub fn render_map(&self, map: &str, position: Vec3, grid_width: f64, grid_height: f64) {
        let map = map_as_vec_of_floors(map);

        let mut map: Vec<&str> = map
            .get((-position.y.div_euclid(grid_height)) as usize)
            .unwrap_or(map.last().unwrap_or(&vec![]))
            .clone();

        let height = map.len();
        let mut width = 0;
        let [pos_x, pos_y] = [
            (position.x.div_euclid(grid_width)) as usize,
            (position.z.div_euclid(grid_width)) as usize,
        ];

        let mut player_string = String::new();

        for (i, row) in map.iter_mut().enumerate() {
            width = width.max(row.len());
        }

        let mut rows = vec![];

        for (i, row) in map.iter_mut().enumerate() {
            rows.push(format!("{: <1$}", row, width));
        }
        for (i, row) in map.iter_mut().enumerate() {
            *row = &rows[i];
        }

        for (i, row) in map.iter_mut().enumerate() {
            if i == pos_y {
                let (p1, p2) = row.split_at_checked(pos_x).unwrap_or((row, " "));
                let (p2, p3) = p2.split_at_checked(1).unwrap_or((p2, " "));
                player_string = format!("{p1}\x1b[48;2;50;255;50m{p2}\x1b[48;2;000;000;000m{p3}");
                *row = &player_string;
                break;
            }
        }

        print!("\x1b[48;2;000;000;000m\x1b[\r");

        let x_start = (self.tw / 2).saturating_sub(width / 2 + 2);
        let y_start = (self.th / 2).saturating_sub(height / 2 + 3);

        println!("\x1b[{};{}H*{:-^3$}*\r", y_start, x_start, "-", width + 4);
        println!(
            "\x1b[{};{}H|{:^3$}|\r",
            y_start + 1,
            x_start,
            " ",
            width + 4
        );
        let y_start = y_start + 2;
        for (i, row) in map.iter().enumerate() {
            //Move cursor
            print!("\x1b[{};{}H", y_start + i, x_start);
            let mut extra = 3;

            if i == pos_y {
                extra += 36;
            }
            //println center aligned string
            println!("| {:<1$}|\r", row, width + extra);
        }
        let y_start = y_start + height;

        println!(
            "\x1b[{};{}H*{:-^3$}*\r",
            y_start + 1,
            x_start,
            "-",
            width + 4
        );
        println!("\x1b[{};{}H|{:^3$}|\r", y_start, x_start, " ", width + 4);
    }
}

pub fn map_as_vec_of_floors(map: &str) -> Vec<Vec<&str>> {
    let spl = map.lines().collect::<Vec<&str>>();
    let mut res = vec![];
    let mut cur = vec![];
    for line in spl {
        if line == "sep" {
            res.push(cur.clone());
            cur = vec![];
        } else {
            cur.push(line);
        }
    }
    res.push(cur.clone());
    res
}
