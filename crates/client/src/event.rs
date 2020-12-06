use common::ChunkPos;
use sdl2::keyboard::Keycode;

/// A chunk has been loaded.
#[derive(Copy, Clone, Debug)]
pub struct ChunkLoaded {
    pub pos: ChunkPos,
}

/// A chunk has been unloaded.
#[derive(Copy, Clone, Debug)]
pub struct ChunkUnloaded {
    pub pos: ChunkPos,
}

/// A key has been pressed.
#[derive(Copy, Clone, Debug)]
pub struct KeyPressed {
    pub key: Keycode,
}

/// A key has been released.
#[derive(Copy, Clone, Debug)]
pub struct KeyReleased {
    pub key: Keycode,
}

/// The mouse has moved.
#[derive(Copy, Clone, Debug)]
pub struct MouseMoved {
    pub xrel: i32,
    pub yrel: i32,
}

/// The window has been resized.
#[derive(Copy, Clone, Debug)]
pub struct WindowResized {
    pub new_width: u32,
    pub new_height: u32,
}
