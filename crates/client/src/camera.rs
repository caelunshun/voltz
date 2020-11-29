use common::{Orient, Pos};
use glam::{Mat4, Vec3};

use crate::game::Game;

const MOUSE_SENSITIVITY: f32 = 4.;
const EYE_HEIGHT: f32 = 5.;

pub struct CameraController;

impl CameraController {
    /// Handles a relative mouse motion event.
    pub fn on_mouse_move(&mut self, game: &mut Game, dx: i32, dy: i32) {
        let dx = dx as f32;
        let dy = dy as f32;

        let mut orient = game.player_ref().get::<Orient>().unwrap().0;
        orient.x -= (MOUSE_SENSITIVITY * dx).to_radians();
        orient.y -= (MOUSE_SENSITIVITY * dy).to_radians();
        game.player_ref().get_mut::<Orient>().unwrap().0 = orient;
    }

    /// Returns the view-projection matrix that should be passed to shaders.
    pub fn view_projection(&mut self, game: &mut Game, aspect_ratio: f32) -> Mat4 {
        let pos = game.player_ref().get::<Pos>().unwrap().0;
        let orient = game.player_ref().get::<Orient>().unwrap().0;

        let eye = pos + glam::vec3a(0., EYE_HEIGHT, 0.);

        // Determine center based on orient
        let direction = glam::vec3(
            orient.x.to_radians().cos() * orient.y.to_radians().cos(),
            orient.y.to_radians().sin(),
            orient.x.to_radians().sin() * orient.y.to_radians().cos(),
        );
        let center = Vec3::from(eye) + direction.normalize();

        let view = Mat4::look_at_lh(eye.into(), center, Vec3::unit_y());
        let projection = Mat4::perspective_lh(70., aspect_ratio, 0.1, 1000.);

        projection * view
    }
}
