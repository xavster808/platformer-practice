use ggez::{
    error::{GameError, GameResult},
    event,
    glam::*,
    graphics::{self, Color},
    input::keyboard::{KeyCode, KeyInput},
    Context,
};

use std::time::{Duration, Instant};
use std::{env, fs, num, path};

const PLAYER_DIMENSION: f32 = 40.0;

const VX_MAX: f32 = 300.0;
const ACCX: f32 = 15.0 * VX_MAX;

const GRAV: f32 = 2500.0;
const JUMPACC: f32 = 0.3125 * GRAV;

pub trait Updatable {
    fn next_state(&self, dt: f32, state: &Level) -> Self;
}

#[derive(Debug)]
struct Rect {
    top_left: Point,
    width: f32,
    height: f32,
}

impl Rect {
    fn is_colliding(&self, other: &Rect) -> bool {
        self.x_overlaps(other) && self.y_overlaps(other)
    }

    fn x_overlaps(&self, other: &Rect) -> bool {
        self.top_left.x < other.top_left.x + other.width
            && self.top_left.x + self.width > other.top_left.x
    }

    fn y_overlaps(&self, other: &Rect) -> bool {
        self.top_left.y > other.top_left.y - other.height
            && self.top_left.y - self.height < other.top_left.y
    }
}

#[derive(Debug, Clone, Copy)]
struct Point {
    x: f32,
    y: f32,
}

impl Point {
    fn new(x_coord: f32, y_coord: f32) -> Point {
        Point {
            x: x_coord,
            y: y_coord,
        }
    }
}

#[derive(Debug)]
struct Control {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

impl Control {
    fn new() -> Control {
        Control {
            up: false,
            down: false,
            left: false,
            right: false,
        }
    }
}

#[derive(Debug)]
struct Player {
    input: Control,
    bounding_box: Rect,
    velocity_x: f32,
    velocity_y: f32,

    spawnpoint: Point,
    deaths: u32,

    coyote_time: f32,
    is_grounded: bool,
    is_grappling: bool,
}

impl Player {
    fn new(size: f32, x: f32, y: f32) -> Player {
        Player {
            input: Control::new(),
            bounding_box: Rect {
                top_left: Point::new(x, y),
                width: size,
                height: size,
            },
            velocity_x: 0.0,
            velocity_y: 0.0,

            spawnpoint: Point::new(x, y),
            deaths: 0,

            coyote_time: 0.0,
            is_grounded: true,
            is_grappling: false,
        }
    }

    // precondition: self's bounding_box must be valid.
    fn evaluate_collisions(&self, guess_box: &Rect, state: &Level) -> (Rect, u32) {
        let mut nudged_box = Rect {
            top_left: Point {
                ..guess_box.top_left
            },
            ..*guess_box
        };
        let mut new_deaths = self.deaths;

        for platform in &state.platforms {
            if nudged_box.is_colliding(&platform.bounding_box) && !platform.lethal {
                if self.bounding_box.y_overlaps(&platform.bounding_box) {
                    // Old y overlapped, x must be changed
                    if self.bounding_box.top_left.x < platform.bounding_box.top_left.x {
                        // If old x was left, snap nuged box left
                        nudged_box.top_left.x = platform.bounding_box.top_left.x - nudged_box.width;
                    } else {
                        // Right case
                        nudged_box.top_left.x =
                            platform.bounding_box.top_left.x + platform.bounding_box.width;
                    }
                } else if self.bounding_box.x_overlaps(&platform.bounding_box) {
                    // If old x overlapped, y must be changed
                    if self.bounding_box.top_left.y > platform.bounding_box.top_left.y {
                        // If old y was above, snap the nudged box to above
                        nudged_box.top_left.y =
                            platform.bounding_box.top_left.y + nudged_box.height;
                    } else {
                        // Below case
                        nudged_box.top_left.y =
                            platform.bounding_box.top_left.y - platform.bounding_box.height;
                    }
                }
            }
        }
        for platform in &state.platforms {
            if platform.lethal && nudged_box.is_colliding(&platform.bounding_box) {
                nudged_box.top_left = self.spawnpoint;
                new_deaths += 1;
                println!("Deaths: {}", new_deaths);
                break;
            }
        }
        (nudged_box, new_deaths)
    }

    fn evaluate_checkpoints(&self, new_box: &Rect, state: &Level) -> Point {
        let mut new_spawnpoint = self.spawnpoint;
        for checkpoint in &state.checkpoints {
            if new_box.is_colliding(&checkpoint.bounding_box) {
                new_spawnpoint = checkpoint.bounding_box.top_left;
                break;
            }
        }
        new_spawnpoint
    }
}

impl Updatable for Player {
    fn next_state(&self, dt: f32, state: &Level) -> Player {
        let mut dvx = 0.0;
        let mut dvy = 0.0;
        let mut new_coyote_time = self.coyote_time;
        if !self.is_grounded {
            dvy -= GRAV;
            new_coyote_time += dt;
        } else {
            new_coyote_time = 0.0;
        }
        let mut new_is_grounded = false;

        let mut new_spawnpoint = self.spawnpoint;
        let mut new_deaths = self.deaths;

        if self.input.right {
            dvx += ACCX;
        } else if self.input.left {
            dvx -= ACCX;
        } else {
            let sign = ((self.velocity_x > 0.0) as i32 - (self.velocity_x < 0.0) as i32) as f32; // sign is positive if velocity_x is negative and vice versa
            dvx += ACCX * sign * -1.0; // This block attracts the player's velocity_x to 0 if idle
        }

        let mut new_velocity_x = self.velocity_x + dvx * dt;
        let mut new_velocity_y = self.velocity_y + dvy * dt;

        if self.input.up && new_coyote_time < 0.08
        /*|| self.is_grounded*/
        {
            new_is_grounded = false;
            new_coyote_time = 10.0; // Out of range; Effectively a jump counter.
            new_velocity_y = JUMPACC;
        }

        if new_velocity_x > VX_MAX {
            new_velocity_x = VX_MAX;
        } else if new_velocity_x < -1.0 * VX_MAX {
            new_velocity_x = -1.0 * VX_MAX
        } else if new_velocity_x.abs() < ACCX * dt {
            new_velocity_x = 0.0;
        }
        if new_velocity_y < -1.0 * JUMPACC {
            new_velocity_y = 0.0 - JUMPACC
        }

        let new_pos_x = self.bounding_box.top_left.x + new_velocity_x * dt;
        let new_pos_y = self.bounding_box.top_left.y + new_velocity_y * dt;
        let mut new_box = Rect {
            top_left: Point::new(new_pos_x, new_pos_y),
            width: self.bounding_box.width,
            height: self.bounding_box.height,
        };

        (new_box, new_deaths) = self.evaluate_collisions(&new_box, state);
        new_spawnpoint = self.evaluate_checkpoints(&new_box, state);

        let ground_test = Rect {
            top_left: Point::new(
                new_box.top_left.x,
                new_box.top_left.y - self.bounding_box.height,
            ),
            width: self.bounding_box.width, //Decreases leniency; 1.0 is too fat
            height: 1.0,
        };
        for platform in &state.platforms {
            if ground_test.is_colliding(&platform.bounding_box) && !platform.lethal {
                new_is_grounded = true;
                break;
            }
        }

        if new_is_grounded || (new_pos_y != new_box.top_left.y) {
            new_velocity_y = 0.0;
        }
        if new_box.top_left.x != new_pos_x {
            new_velocity_x = 0.0;
        }

        Player {
            input: Control {
                up: self.input.up,
                down: self.input.down,
                left: self.input.left,
                right: self.input.right,
            },
            bounding_box: new_box,
            velocity_x: new_velocity_x,
            velocity_y: new_velocity_y,

            spawnpoint: new_spawnpoint,
            deaths: new_deaths,

            coyote_time: new_coyote_time,
            is_grounded: new_is_grounded,
            is_grappling: false,
        }
    }
}

struct Platform {
    bounding_box: Rect,

    lethal: bool,
}

impl Platform {
    fn read_platforms(path: &str) -> Vec<Platform> {
        let level = fs::read_to_string(path).expect("Expected readable file");
        let level = level.as_bytes();
        let mut retval: Vec<Platform> = Vec::new();

        let plat_symbols = vec![0x3D, 0x78]; // =, x
        let unit = PLAYER_DIMENSION / 2.0;

        let mut left_offset = 0;
        let mut down_offset = 1;
        let mut length = 1;
        let mut previous_char = &0x00;

        for c in level {
            if c == previous_char {
                length += 1;
            } else {
                if plat_symbols.contains(previous_char) {
                    retval.push(Platform {
                        bounding_box: Rect {
                            top_left: Point::new(
                                unit * (left_offset - length) as f32,
                                unit * down_offset as f32,
                            ),
                            width: unit * length as f32,
                            height: unit,
                        },
                        lethal: if previous_char == &0x3D { false } else { true },
                    });
                }

                length = 1;

                if c == &0x0a {
                    // "\n"
                    left_offset = -1;
                    down_offset += 1;
                }
            }
            left_offset += 1;
            previous_char = c;
        }
        retval
    }
}

struct Checkpoint {
    bounding_box: Rect,
}

impl Checkpoint {
    fn read_checkpoints(path: &str) -> Vec<Checkpoint> {
        let level = fs::read_to_string(path).expect("Expected readable file");
        let level = level.as_bytes();
        let mut retval: Vec<Checkpoint> = Vec::new();

        let plat_symbols = vec![0x63]; // c
        let unit = PLAYER_DIMENSION / 2.0;

        let mut left_offset = 0;
        let mut down_offset = 1;

        for c in level {
            if plat_symbols.contains(c) {
                retval.push(Checkpoint {
                    bounding_box: Rect {
                        top_left: Point::new(
                            unit * (left_offset) as f32,
                            unit * down_offset as f32,
                        ),
                        width: unit as f32,
                        height: unit as f32,
                    },
                });
            }
            if c == &0x0a {
                // "\n"
                left_offset = -1;
                down_offset += 1;
            }
            left_offset += 1;
        }
        retval
    }
}

struct Level {
    player: Player,
    platforms: Vec<Platform>,
    checkpoints: Vec<Checkpoint>,

    last_update: Instant,
}

impl Level {
    fn new(ctx: &mut Context) -> GameResult<Level> {
        ctx.gfx.add_font(
            "LiberationMono",
            graphics::FontData::from_path(ctx, "/LiberationMono-Regular.ttf")?,
        );
        let mc = Player::new(PLAYER_DIMENSION, 100.0, 300.0);

        let file = "level2.txt";
        let l = Level {
            player: mc,
            platforms: Platform::read_platforms(file),
            checkpoints: Checkpoint::read_checkpoints(file),
            last_update: Instant::now(),
        };
        Ok(l)
    }
}

impl event::EventHandler for Level {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        self.player = self.player.next_state(dt, &self);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        //println!("{:#?}", self.player);

        let mut canvas =
            graphics::Canvas::from_frame(ctx, graphics::Color::from([0.4, 0.3, 0.3, 1.0]));

        let x_offset = if self.player.bounding_box.top_left.x >= 0.0 {
            (self.player.bounding_box.top_left.x as i32 / 800) as f32 * 800.0
        } else {
            ((self.player.bounding_box.top_left.x as i32 + 1) / 800 - 1) as f32 * 800.0
        };
        let y_offset = if self.player.bounding_box.top_left.y >= 0.0 {
            (self.player.bounding_box.top_left.y as i32 / 600) as f32 * 600.0
        } else {
            ((self.player.bounding_box.top_left.y as i32 + 1) / 600 - 1) as f32 * 600.0
        };

        let mc = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            graphics::Rect::new(
                0.0 - x_offset,
                0.0 + y_offset,
                self.player.bounding_box.width,
                self.player.bounding_box.height,
            ),
            Color::BLUE,
        )?;

        canvas.draw(
            &mc,
            Vec2::new(
                self.player.bounding_box.top_left.x,
                600.0 - self.player.bounding_box.top_left.y,
            ),
        );

        for platform in &self.platforms {
            let plat = graphics::Mesh::new_rectangle(
                ctx,
                graphics::DrawMode::fill(),
                graphics::Rect::new(
                    0.0 - x_offset,
                    0.0 + y_offset,
                    platform.bounding_box.width,
                    platform.bounding_box.height,
                ),
                if platform.lethal {
                    Color::RED
                } else {
                    Color::new(0.2, 0.2, 0.2, 1.0)
                },
            )?;
            canvas.draw(
                &plat,
                Vec2::new(
                    platform.bounding_box.top_left.x,
                    600.0 - platform.bounding_box.top_left.y,
                ),
            )
        }

        for checkpoint in &self.checkpoints {
            let checky = graphics::Mesh::new_rectangle(
                ctx,
                graphics::DrawMode::fill(),
                graphics::Rect::new(
                    0.0 - x_offset,
                    0.0 + y_offset,
                    checkpoint.bounding_box.width,
                    checkpoint.bounding_box.height,
                ),
                Color::new(0.5, 0.8, 0.5, 0.4),
            )?;
            canvas.draw(
                &checky,
                Vec2::new(
                    checkpoint.bounding_box.top_left.x,
                    600.0 - checkpoint.bounding_box.top_left.y,
                ),
            )
        }

        canvas.finish(ctx)?;

        Ok(())
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        input: KeyInput,
        _repeated: bool,
    ) -> GameResult {
        match input.keycode {
            Some(KeyCode::Up) => self.player.input.up = true,
            Some(KeyCode::Down) => self.player.input.down = true,
            Some(KeyCode::Left) => self.player.input.left = true,
            Some(KeyCode::Right) => self.player.input.right = true,
            _ => {} // Ignore other key presses
        }
        Ok(())
    }

    fn key_up_event(&mut self, ctx: &mut Context, input: KeyInput) -> GameResult {
        match input.keycode {
            Some(KeyCode::Up) => self.player.input.up = false,
            Some(KeyCode::Down) => self.player.input.down = false,
            Some(KeyCode::Left) => self.player.input.left = false,
            Some(KeyCode::Right) => self.player.input.right = false,
            _ => {} // Ignore other key presses
        }
        Ok(())
    }
}

fn main() -> GameResult {
    let resource_dir = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        path
    } else {
        path::PathBuf::from("./resources")
    };

    let cb = ggez::ContextBuilder::new("helloworld", "XS").add_resource_path(resource_dir);
    let (mut ctx, event_loop) = cb.build()?;

    let level = Level::new(&mut ctx)?;
    event::run(ctx, event_loop, level)
}