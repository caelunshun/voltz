use glam::Vec2;
use serde::{Deserialize, Serialize};

/// A rectangle.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct Rect {
    pub pos: Vec2,
    pub size: Vec2,
}

/// A color in linear RGBA space.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1. }
    }
}
