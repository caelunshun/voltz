use std::{collections::HashMap, iter::once};

use serde::{Deserialize, Serialize};

/// A block model loaded from asset/model/block/*.yml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlModel {
    /// Optionally inherit from the model with the given path.
    #[serde(default)]
    pub inherits: Option<String>,
    /// Texture variables which are declared but not set to a value.
    /// Used for inheritance.
    #[serde(default)]
    pub texture_parameters: HashMap<String, TextureParam>,
    /// Initialize texture parameters (potentially those of the parent).
    #[serde(default)]
    pub textures: HashMap<String, String>,
    /// A list of rectangular prisms which define this block model.
    #[serde(default)]
    pub prisms: Vec<Prism>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureParam {
    /// A default texture parameter to defer to if
    /// this parameter is not set.
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prism {
    /// The faces of this prism and their textures.
    pub faces: Faces,
    /// The dimensions of the prism on each axis.
    /// Measured in 1/64 of a block.
    pub extent: Extent,
    /// The offset from (0, 0, 0) within the block.
    pub offset: Offset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faces {
    pub top: Face,
    pub bottom: Face,
    pub posx: Face,
    pub negx: Face,
    pub posz: Face,
    pub negz: Face,
}

impl Faces {
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Face> {
        once(&self.top)
            .chain(once(&self.bottom))
            .chain(once(&self.posx))
            .chain(once(&self.negx))
            .chain(once(&self.posz))
            .chain(once(&self.negz))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Face {
    /// The texture to use for this face.
    pub texture: String,
}

/// Measured in 1/64 of a block.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Extent {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl From<Extent> for [u8; 3] {
    fn from(e: Extent) -> Self {
        [e.x, e.y, e.z]
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Offset {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl From<Offset> for [u8; 3] {
    fn from(o: Offset) -> Self {
        [o.x, o.y, o.z]
    }
}
