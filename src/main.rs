use ggez::{event, graphics, Context, GameResult};
use ggez::error::GameError;
use ggez::input::keyboard::{KeyInput, KeyCode, KeyMods};

use std::{env, path, num};
use std::time::{Duration, Instant};



const VX_MAX: f32 = 3.0;
const ACCX: f32 = 8.0 * VX_MAX;

const GRAV: f32 = 50.0;
// const ACCY: f32 = 0.3 * GRAV;
const JUMPACC: f32 = 0.4 * GRAV;

pub trait Updatable {
    fn update_entity(&mut self, dt: f32) {}
    // fn is_colliding();
}
/*
pub trait Resource<T> {
    fn load(ctx: &mut Context, path: &str) -> Result<T, GameError>;
}

impl Resource<graphics::Image> for graphics::Image {
    fn load(ctx: &mut Context, path: &str) -> Result<Self, GameError> {
        graphics::Image::new(ctx, path)
    }
}

struct ResourceManager {
    assets: HashMap<String, Box<dyn Resource>>,
}
*/

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
    pos: Point,
    width: f32,
    height: f32,
    velocity_x: f32,
    velocity_y: f32,
    
    is_grounded: bool,
    is_grappling: bool,
}

impl Player {
    fn new() -> Player {
        Player {
            input: Control::new(),
            pos: Point{
                x: 0.0,
                y: 0.0,
            },
            width: 50.0,
            height: 50.0,
            velocity_x: 0.0,
            velocity_y: 0.0,

            is_grounded: true,
            is_grappling: false,
        }
    }

    fn update_is_grounded(&mut self) {
        self.is_grounded = self.pos.y <= 0.0;
    }

    fn y_is_valid(y_coord: f32) -> bool {
        y_coord > 0.0
    }
}

impl Updatable for Player {
    fn update_entity(&mut self, dt: f32) {
        self.update_is_grounded();
        
        let mut dvx = 0.0;
        let mut dvy = 0.0;
        
        if self.input.right {
            dvx += ACCX;
        } else if self.input.left {
            dvx -= ACCX;
        } else {
            let sign = ((self.velocity_x > 0.0) as i32 - (self.velocity_x < 0.0) as i32) as f32 ; // sign is possitive if velocity_x is negative and vice versa
            dvx = ACCX * sign * -1.0;                                                             // This block attracts the player's velocity_x to 0 if idle
        }

        if !self.is_grounded {
            dvy -= GRAV;
        } else if self.is_grounded && self.input.up {
            dvy += JUMPACC / dt;
        }

        self.velocity_x += dvx * dt;
        self.velocity_y += dvy * dt;

//        if self.velocity_x > VX_MAX || self.velocity_x < -1.0 * VX_MAX as f32 {
//            self.velocity_x -= dvx * dt;

        if self.velocity_x > VX_MAX {
            self.velocity_x = VX_MAX;
        } else if self.velocity_x < -1.0 * VX_MAX {
            self.velocity_x = -1.0 * VX_MAX
        } else if self.velocity_x.abs() < ACCX * dt {
            self.velocity_x = 0.0;
        }

        let new_pos_x = self.pos.x + self.velocity_x * dt;
        let new_pos_y = self.pos.y + self.velocity_y * dt;
        
        self.pos.x = new_pos_x; // Add collision checks
        if Player::y_is_valid(new_pos_y) {
            if new_pos_y < self.velocity_y * dt * 1.1 && self.velocity_y < 0.0 {
                self.pos.y = 0.0;
            } else {
                self.pos.y = new_pos_y;
            }
        } else {
            self.velocity_y = 0.0;
            self.pos.y = 0.0;
        }
    }
}

struct Platform {
    pos: Point,
    width: f32,
    height: f32,
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
        let mc = Player::new();
        let platform = Platform {
            pos: Point::new(500.0, 20.0),
            width: 1000.0,
            height: 40.0,
        };
        let l = Level {
            player: mc,
            platforms: vec![platform],
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
        
        self.player.update_entity(dt);

        Ok(())
    }

    fn draw(& mut self, ctx: &mut Context) -> GameResult {
        println!("{:#?}", self.player); 
        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, input: KeyInput, _repeated: bool) -> GameResult {
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
