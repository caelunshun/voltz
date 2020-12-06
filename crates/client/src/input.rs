//! Takes `winit` input and writes it to the event bus.

use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, WindowEvent},
};

use crate::{
    event::{KeyPressed, KeyReleased, MouseMoved, WindowResized},
    game::Game,
};

pub fn handle_event(event: &WindowEvent, game: &mut Game) {
    match event {
        WindowEvent::Resized(new_size) => game.events().push(WindowResized {
            new_width: new_size.width,
            new_height: new_size.height,
        }),
        WindowEvent::KeyboardInput { input, .. } => match input.state {
            ElementState::Pressed => {
                if let Some(key) = input.virtual_keycode {
                    game.events().push(KeyPressed { key });
                    game.insert_pressed_key(key);
                }
            }
            ElementState::Released => {
                if let Some(key) = input.virtual_keycode {
                    game.events().push(KeyReleased { key });
                    game.remove_pressed_key(key);
                }
            }
        },
        WindowEvent::CursorMoved { position, .. } => {
            let size = game.window().inner_size();
            game.events().push(MouseMoved {
                xrel: ((position.x - game.mouse_pos.x) / size.width as f64) * 1000.,
                yrel: ((position.y - game.mouse_pos.y) / size.height as f64) * 1000.,
            });
            let mouse_pos = PhysicalPosition::new(size.width as f64 / 2., size.height as f64 / 2.);
            game.mouse_pos = mouse_pos;
            if let Err(e) = game.window_mut().set_cursor_position(mouse_pos) {
                log::error!("Failed to set cursor position: {:?}", e);
            }
        }
        _ => (),
    }
}
