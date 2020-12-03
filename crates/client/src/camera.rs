use crate::{game::Game, PLAYER_BBOX};
use bytemuck::{Pod, Zeroable};
use common::{blocks, entity::Vel, BlockId, Orient, Pos};
use glam::{Mat4, Vec2, Vec3, Vec3A};
use sdl2::keyboard::{KeyboardState, Scancode};
use splines::{Interpolation, Key, Spline};

const MOUSE_SENSITIVITY: f32 = 4.;
const KEYBOARD_SENSITIVITY: f32 = 6.;
const EYE_HEIGHT: f32 = 1.6;

const JUMP_VEL_Y: f32 = 8.;

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
pub struct Matrices {
    pub view: Mat4,
    pub projection: Mat4,
}

pub struct CameraController {
    move_spline: Spline<f32, f32>,
    move_time: Option<f32>,
    stop_time: f32,
}

impl CameraController {
    pub fn new() -> Self {
        let move_spline = Spline::from_iter(
            [
                Key {
                    t: 0.,
                    value: 0.,
                    interpolation: Interpolation::Bezier(0.1),
                },
                Key {
                    t: 0.2,
                    value: 1.,
                    interpolation: Interpolation::Bezier(0.8),
                },
            ]
            .iter()
            .copied(),
        );
        let move_time = None;

        Self {
            move_spline,
            move_time,
            stop_time: f32::INFINITY,
        }
    }

    /// Handles a relative mouse motion event.
    pub fn on_mouse_move(&mut self, game: &mut Game, dx: i32, dy: i32) {
        let dx = dx as f32;
        let dy = dy as f32;

        let mut orient = game.player_ref().get::<Orient>().unwrap().0;
        orient.x -= (MOUSE_SENSITIVITY * dx).to_radians();
        orient.y -= (MOUSE_SENSITIVITY * dy).to_radians();
        game.player_ref().get_mut::<Orient>().unwrap().0 = orient;
    }

    /// Called each frame to update position based on keyboard actions.
    pub fn tick_keyboard(&mut self, game: &mut Game, keyboard: KeyboardState) {
        self.tick_move(game, &keyboard);
        self.tick_jump(game, &keyboard);
    }

    fn tick_move(&mut self, game: &mut Game, keyboard: &KeyboardState) {
        let orient = game.player_ref().get::<Orient>().unwrap().0;
        let forward = Vec3A::from(self.direction(orient));
        let right = Vec3A::from(forward.cross(Vec3A::unit_y())).normalize();

        let mut vel = Vec3A::zero();
        let mut moved = false;
        if keyboard.is_scancode_pressed(Scancode::W) {
            vel += KEYBOARD_SENSITIVITY * forward;
            moved = true;
        }
        if keyboard.is_scancode_pressed(Scancode::S) {
            vel -= KEYBOARD_SENSITIVITY * forward;
            moved = true;
        }
        if keyboard.is_scancode_pressed(Scancode::A) {
            vel += KEYBOARD_SENSITIVITY * right;
            moved = true;
        }
        if keyboard.is_scancode_pressed(Scancode::D) {
            vel -= KEYBOARD_SENSITIVITY * right;
            moved = true;
        }

        vel.y = 0.;
        vel *= game.dt();

        let multiplier = if moved {
            let time = match &mut self.move_time {
                Some(time) => {
                    *time += game.dt();
                    *time
                }
                None => {
                    self.move_time = Some(game.dt());
                    self.stop_time = 0.;
                    game.dt()
                }
            };
            self.move_spline.clamped_sample(time).unwrap()
        } else {
            self.move_time = None;
            self.stop_time += game.dt();
            if let Some(speed) = self
                .move_spline
                .sample(self.move_spline.keys().last().unwrap().t - self.stop_time)
            {
                vel = forward * speed * KEYBOARD_SENSITIVITY * game.dt();
                1.
            } else {
                0.
            }
        };
        vel *= multiplier;

        let old_pos = game.player_ref().get::<Pos>().unwrap().0;
        let new_pos = old_pos + vel;
        let new_pos =
            physics::collision::resolve_collisions(PLAYER_BBOX, old_pos, new_pos, |pos| {
                game.main_zone().block(pos) != Some(BlockId::new(blocks::Air))
            });
        game.player_ref().get_mut::<Pos>().unwrap().0 = new_pos;
    }

    fn tick_jump(&mut self, game: &mut Game, keyboard: &KeyboardState) {
        if keyboard.is_scancode_pressed(Scancode::Space)
            && physics::is_on_ground(game.player_ref().get::<Pos>().unwrap().0, |pos| {
                game.main_zone().block(pos) != Some(BlockId::new(blocks::Air))
            })
        {
            let vel = glam::vec3a(0., JUMP_VEL_Y, 0.);
            game.player_ref().get_mut::<Vel>().unwrap().0 = vel;
            log::trace!("Jumped - applying velocity {:?}", vel);
        }
    }

    /// Returns the view-projection matrix that should be passed to shaders.
    pub fn matrices(&mut self, game: &mut Game, aspect_ratio: f32) -> Matrices {
        let pos = game.player_ref().get::<Pos>().unwrap().0;
        let orient = game.player_ref().get::<Orient>().unwrap().0;

        let eye = pos + glam::vec3a(0., EYE_HEIGHT, 0.);

        // Determine center based on orient
        let direction = self.direction(orient);
        let center = Vec3::from(eye) + direction;

        let view = Mat4::look_at_lh(eye.into(), center, Vec3::unit_y());
        let projection = Mat4::perspective_lh(70., aspect_ratio, 0.01, 1000.);

        Matrices { view, projection }
    }

    /// Determines the direction vector of the player.
    fn direction(&self, orient: Vec2) -> Vec3 {
        glam::vec3(
            orient.x.to_radians().cos() * orient.y.to_radians().cos(),
            orient.y.to_radians().sin(),
            orient.x.to_radians().sin() * orient.y.to_radians().cos(),
        )
        .normalize()
    }
}
