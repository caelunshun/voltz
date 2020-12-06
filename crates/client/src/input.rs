//! Takes SDL2 input and writes it to the event bus.

use common::SystemExecutor;
use sdl2::event::{Event, WindowEvent};

use crate::{
    event::{KeyPressed, KeyReleased, MouseMoved, WindowResized},
    game::Game,
};

pub fn setup(systems: &mut SystemExecutor<Game>) {
    systems.add(input_system);
}

fn input_system(game: &mut Game) {
    let mut pressed = Vec::new();
    let mut released = Vec::new();
    let mut bus = game.events();
    for event in game.event_pump().poll_iter() {
        match event {
            Event::Window { win_event, .. } => match win_event {
                WindowEvent::Resized(new_width, new_height) => bus.push(WindowResized {
                    new_width: new_width as u32,
                    new_height: new_height as u32,
                }),
                WindowEvent::Close => game.close(),
                _ => (),
            },
            Event::KeyDown { keycode, .. } => {
                if let Some(key) = keycode {
                    bus.push(KeyPressed { key });
                    pressed.push(key);
                }
            }
            Event::KeyUp { keycode, .. } => {
                if let Some(key) = keycode {
                    bus.push(KeyReleased { key });
                    released.push(key);
                }
            }
            Event::MouseMotion { xrel, yrel, .. } => bus.push(MouseMoved { xrel, yrel }),
            _ => (),
        }
    }

    drop(bus);

    for pressed in pressed {
        game.insert_pressed_key(pressed);
    }
    for released in released {
        game.remove_pressed_key(released);
    }
}
