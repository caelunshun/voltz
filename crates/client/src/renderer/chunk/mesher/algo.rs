//! The implementation for offset: (), extent: (), textures: () offset: (), extent: (), textures: () the chunk mesher algorithm.

use bumpalo::Bump;
use common::{chunk::CHUNK_DIM, chunk::CHUNK_VOLUME, BlockId, Chunk};
use glam::Vec3;
use utils::BitSet;

use super::{
    compile::{CompiledModel, Prism},
    Mesher,
};

/// A generated chunk mesh.
#[derive(Debug)]
pub struct Mesh<'bump> {
    pub vertices: Vec<RawVertex, &'bump Bump>,
}

impl Mesh<'_> {
    pub fn push_prism(&mut self, prism: &Prism, offset: Vec3) {
        // TODO: figure out how to move this into a function.
        let offset = offset + vec3(prism.offset);
        let size = vec3(prism.extent);
        let quads = [
            // Top face
            [
                RawVertex {
                    pos: offset + size * glam::vec3(0., 1., 0.),
                    texcoord: glam::vec3(0., 0., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(1., 1., 0.),
                    texcoord: glam::vec3(1., 0., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(1., 1., 1.),
                    texcoord: glam::vec3(1., 1., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(0., 1., 1.),
                    texcoord: glam::vec3(0., 1., prism.textures[0] as f32),
                },
            ],
            // Bottom face
            [
                RawVertex {
                    pos: offset,
                    texcoord: glam::vec3(0., 0., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(1., 0., 0.),
                    texcoord: glam::vec3(1., 0., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(1., 0., 1.),
                    texcoord: glam::vec3(1., 1., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(0., 0., 1.),
                    texcoord: glam::vec3(0., 1., prism.textures[0] as f32),
                },
            ],
            // Positive X
            [
                RawVertex {
                    pos: offset + size * glam::vec3(1., 0., 0.),
                    texcoord: glam::vec3(0., 0., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(1., 0., 1.),
                    texcoord: glam::vec3(1., 0., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(1., 1., 1.),
                    texcoord: glam::vec3(1., 1., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(1., 1., 0.),
                    texcoord: glam::vec3(0., 1., prism.textures[0] as f32),
                },
            ],
            // Negative X
            [
                RawVertex {
                    pos: offset,
                    texcoord: glam::vec3(0., 0., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(0., 0., 1.),
                    texcoord: glam::vec3(1., 0., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(0., 1., 1.),
                    texcoord: glam::vec3(1., 1., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(0., 1., 0.),
                    texcoord: glam::vec3(0., 1., prism.textures[0] as f32),
                },
            ],
            // Positive Z
            [
                RawVertex {
                    pos: offset + size * glam::vec3(0., 0., 1.),
                    texcoord: glam::vec3(0., 0., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(1., 0., 1.),
                    texcoord: glam::vec3(1., 0., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(1., 1., 1.),
                    texcoord: glam::vec3(1., 1., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(0., 1., 1.),
                    texcoord: glam::vec3(0., 1., prism.textures[0] as f32),
                },
            ],
            // Negative Z
            [
                RawVertex {
                    pos: offset,
                    texcoord: glam::vec3(0., 0., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(1., 0., 0.),
                    texcoord: glam::vec3(1., 0., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(1., 1., 0.),
                    texcoord: glam::vec3(1., 1., prism.textures[0] as f32),
                },
                RawVertex {
                    pos: offset + size * glam::vec3(0., 1., 0.),
                    texcoord: glam::vec3(0., 1., prism.textures[0] as f32),
                },
            ],
        ];
        for &quad in &quads {
            self.push_quad(quad);
        }
    }

    pub fn push_quad(&mut self, vertices: [RawVertex; 4]) {
        self.vertices.extend_from_slice(&[
            vertices[0],
            vertices[1],
            vertices[2],
            vertices[2],
            vertices[3],
            vertices[0],
        ]);
    }

    pub fn to_obj(&self) -> String {
        self.vertices
            .chunks_exact(3)
            .enumerate()
            .map(|(i, tri)| {
                let vertices = tri
                    .iter()
                    .map(|v| format!("v {} {} {}\n", v.pos.x, v.pos.y, v.pos.z))
                    .collect::<String>();
                format!("{}f {} {} {}\n", vertices, i * 3 + 1, i * 3 + 2, i * 3 + 3)
            })
            .collect::<String>()
    }
}

fn vec3(in_steps: [u8; 3]) -> Vec3 {
    Vec3::new(
        in_steps[0] as f32 / 64.,
        in_steps[1] as f32 / 64.,
        in_steps[2] as f32 / 64.,
    )
}

#[derive(Copy, Clone, Debug)]
pub struct RawVertex {
    pub pos: Vec3,
    pub texcoord: Vec3,
}

struct State<'a> {
    chunk: &'a Chunk,
    bump: &'a Bump,

    mesh: Mesh<'a>,

    /// The blocks which still have to be processed.
    /// Ordered the same way as `Chunk::indexes()`.
    remaining: BitSet<&'a Bump>,
}

impl<'a> State<'a> {
    pub fn mark_finished(&mut self, pos: [usize; 3]) {
        let index = pos[1] * CHUNK_DIM * CHUNK_DIM + pos[2] * CHUNK_DIM + pos[0];
        self.remaining.remove(index as usize);
    }
}

/// Gets the function used to mesh a given block
/// using a model.
/// The returned function takes as input:
/// * The mesher [`State`]
/// * The position of the block relative to the chunk origin
///
/// The function will process one _or more_ blocks,
/// remove them from the `remaining` set, and add
/// the resulting vertices to the output mesh.
///
/// Different functions are used for different models
/// as specializations. For example, a model which is a solid
/// block is meshed using a greedy meshing algorithm. An empty
/// model uses a no-op function, and a complex model uses
/// a naive implementation which copies the model's vertices
/// into the mesh.

// TODO: use Box<T, &Bump> once https://github.com/rust-lang/rust/issues/78459 is fixed.

fn mesh_function<'a, 'bump>(
    model: &'a CompiledModel,
    _block: BlockId,
    _bump: &'bump Bump,
) -> Box<dyn FnMut(&mut State, [usize; 3]) + 'a> {
    if model.prisms.is_empty() {
        Box::new(mesh_noop)
    } else {
        Box::new(move |state, pos| mesh_naive(state, pos, &model.prisms))
    }
}

/// Mesher function which just clears the block from
/// the `remaining` set. Effectively a no-op.
fn mesh_noop(state: &mut State, pos: [usize; 3]) {
    state.mark_finished(pos);
}

/// Mesher function which copies a set of prisms
/// into the mesh. Used for nontrivial models
/// (i.e., those that are neither full cubes or
/// empty).
fn mesh_naive(state: &mut State, pos: [usize; 3], prisms: &[Prism]) {
    let offset = Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);

    for prism in prisms {
        state.mesh.push_prism(prism, offset);
    }

    state.mark_finished(pos);
}

/// Meshes a chunk: converts a volume of blocks to a [`Mesh`].
pub(super) fn mesh<'bump>(mesher: &Mesher, chunk: &'bump Chunk, bump: &'bump Bump) -> Mesh<'bump> {
    let mesh = Mesh {
        vertices: Vec::new_in(bump),
    };
    let mut remaining = BitSet::new_in(CHUNK_VOLUME, bump);
    remaining.fill();
    let mut state = State {
        chunk,
        bump,
        mesh,
        remaining,
    };

    let mut mesh_fns = Vec::new_in(bump);
    mesh_fns.extend(chunk.palette().iter().copied().map(|block| {
        let model = mesher
            .models
            .get(block.descriptor().slug())
            .unwrap_or_else(|| mesher.models.get("unknown").expect("missing unknown model"));
        mesh_function(model, block, bump)
    }));

    let indexes = chunk.indexes();
    let mut pos = 0;
    while let Some(next_pos) = state.remaining.next(pos) {
        pos = next_pos;

        let palette_index = indexes.get(pos).expect("out of bounds");
        let mesh = &mut mesh_fns[palette_index as usize];
        let y = pos / (CHUNK_DIM * CHUNK_DIM);
        let z = (pos / CHUNK_DIM) - (y * CHUNK_DIM);
        let x = pos % CHUNK_DIM;
        mesh(&mut state, [x, y, z]);
    }

    state.mesh
}

#[cfg(test)]
mod tests {
    /*
    use std::fs;

    use ahash::AHashMap;
    use common::blocks;

    use super::*;


    #[test]
    fn dump_mesh() {
        let mut chunk = Chunk::new();
        for y in 0..8 {
            for x in 0..16 {
                for z in 0..16 {
                    chunk.set(x, y, z, BlockId::new(blocks::Stone));
                }
            }
        }

        let mut models = AHashMap::new();
        models.insert(
            "unknown".to_owned(),
            CompiledModel {
                prisms: vec![Prism {
                    offset: [0, 0, 0],
                    extent: [64, 64, 64],
                    textures: [0, 0, 0, 0, 0, 0],
                }],
            },
        );
        let mesher = Mesher { models };

        let bump = Bump::new();
        let mesh = mesh(&mesher, &chunk, &bump);

        let obj = mesh.to_obj();
        fs::write("mesh.obj", obj.as_bytes()).unwrap();
    }
    */
}
