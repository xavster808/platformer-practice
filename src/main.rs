use ggez::{
    error::{GameError, GameResult},
    event,
    glam::*,
    graphics::{self, Color},
    input::keyboard::{KeyCode, KeyInput, KeyMods},
    Context,
};

use std::time::{Duration, Instant};
use std::{env, num, path};

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

#[derive(Debug)]
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

            is_grounded: true,
            is_grappling: false,
        }
    }

    // precondition: self's bounding_box must be valid.
    fn evaluate_collisions(&self, guess_box: &Rect, state: &Level) -> Rect {
        let mut nudged_box = Rect {
            top_left: Point {
                ..guess_box.top_left
            },
            ..*guess_box
        };

        for platform in &state.platforms {
            if guess_box.is_colliding(&platform.bounding_box) {
                if self.bounding_box.y_overlaps(&platform.bounding_box) {
                    // Old y overlapped, x must be changed
                    if self.bounding_box.top_left.x < platform.bounding_box.top_left.x {
                        // If old x was left, snap nuged box left
                        nudged_box.top_left.x = platform.bounding_box.top_left.x - nudged_box.width;
                    } else {
                        // Right case
                        nudged_box.top_left.x =
                            platform.bounding_box.top_left.x + platform.bounding_box.height;
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

        nudged_box
    }
}

impl Updatable for Player {
    fn next_state(&self, dt: f32, state: &Level) -> Player {
        let mut dvx = 0.0;
        let mut dvy = GRAV * -1.0;

        let mut new_is_grounded = self.is_grounded;

        if self.input.right {
            dvx += ACCX;
        } else if self.input.left {
            dvx -= ACCX;
        } else {
            let sign = ((self.velocity_x > 0.0) as i32 - (self.velocity_x < 0.0) as i32) as f32; // sign is possitive if velocity_x is negative and vice versa
            dvx = ACCX * sign * -1.0; // This block attracts the player's velocity_x to 0 if idle
        }

        if self.is_grounded && self.input.up {
            dvy += JUMPACC / dt;
            new_is_grounded = false;
        }

        let mut new_velocity_x = self.velocity_x + dvx * dt;
        let mut new_velocity_y = self.velocity_y + dvy * dt;

        if new_velocity_x > VX_MAX {
            new_velocity_x = VX_MAX;
        } else if new_velocity_x < -1.0 * VX_MAX {
            new_velocity_x = -1.0 * VX_MAX
        } else if new_velocity_x.abs() < ACCX * dt {
            new_velocity_x = 0.0;
        }

        let new_pos_x = self.bounding_box.top_left.x + new_velocity_x * dt;
        let new_pos_y = self.bounding_box.top_left.y + new_velocity_y * dt;
        let mut new_box = Rect {
            top_left: Point::new(new_pos_x, new_pos_y),
            width: self.bounding_box.width,
            height: self.bounding_box.height,
        };

        new_box = self.evaluate_collisions(&new_box, state);

        if new_box.top_left.y != new_pos_y {
            new_velocity_y = 0.0;
            if new_velocity_y <= 0.0 {
                new_is_grounded = true;
            }
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

            is_grounded: new_is_grounded,
            is_grappling: false,
        }
    }
}

struct Platform {
    bounding_box: Rect,

    lethal: bool,
}

struct Level {
    player: Player,
    platforms: Vec<Platform>,

    last_update: Instant,
}

impl Level {
    fn new(ctx: &mut Context) -> GameResult<Level> {
        ctx.gfx.add_font(
            "LiberationMono",
            graphics::FontData::from_path(ctx, "/LiberationMono-Regular.ttf")?,
        );
        let mc = Player::new(40.0, 200.0, 200.0);
        let platform1 = Platform {
            bounding_box: Rect {
                top_left: Point::new(100.0, 100.0),
                width: 300.0,
                height: 40.0,
            },
            lethal: false,
        };
        let platform2 = Platform {
            bounding_box: Rect {
                top_left: Point::new(200.0, 200.0),
                width: 300.0,
                height: 40.0,
            },
            lethal: false,
        };
        let l = Level {
            player: mc,
            platforms: vec![platform1, platform2],
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
        println!("{:#?}", self.player);

        let mut canvas =
            graphics::Canvas::from_frame(ctx, graphics::Color::from([0.2, 0.3, 0.4, 1.0]));
        
        let mc = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            graphics::Rect::new(
            0.0,
            0.0,
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
                    0.0,
                    0.0,
                    platform.bounding_box.width,
                    platform.bounding_box.height,
                ),
                Color::YELLOW,
            )?;
            canvas.draw(
                &plat,
                Vec2::new(
                    platform.bounding_box.top_left.x,
                    600.0 - platform.bounding_box.top_left.y,
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
