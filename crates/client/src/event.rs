use common::ChunkPos;
use winit::event::VirtualKeyCode;

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
    pub key: VirtualKeyCode,
}

/// A key has been released.
#[derive(Copy, Clone, Debug)]
pub struct KeyReleased {
    pub key: VirtualKeyCode,
}

/// The mouse has moved.
#[derive(Copy, Clone, Debug)]
pub struct MouseMoved {
    pub xrel: f64,
    pub yrel: f64,
}

/// The window has been resized.
#[derive(Copy, Clone, Debug)]
pub struct WindowResized {
    pub new_width: u32,
    pub new_height: u32,
}
