//! The implementation for the chunk mesher algorithm.

use ahash::AHashMap;
use bumpalo::Bump;
use common::{blocks, chunk::CHUNK_DIM, chunk::CHUNK_VOLUME, BlockId, Chunk};
use glam::{Vec2, Vec3, Vec3Swizzles};
use utils::BitSet;

use super::compile::{CompiledModel, Prism};

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

        self.push_cube(offset, size, prism.textures);
    }

    pub fn push_cube(&mut self, offset: Vec3, size: Vec3, textures: [u32; 6]) {
        let x0y0z0 = offset;
        let x1y0z0 = offset + size * glam::vec3(1., 0., 0.);
        let x1y0z1 = offset + size * glam::vec3(1., 0., 1.);
        let x0y0z1 = offset + size * glam::vec3(0., 0., 1.);

        let x0y1z0 = offset + size * glam::vec3(0., 1., 0.);
        let x1y1z0 = offset + size * glam::vec3(1., 1., 0.);
        let x1y1z1 = offset + size * glam::vec3(1., 1., 1.);
        let x0y1z1 = offset + size * glam::vec3(0., 1., 1.);

        fn quad(corners: &[Vec3; 4], size: Vec2, normal: Vec3, texture: f32) -> [RawVertex; 4] {
            let size = glam::vec3(size.x, size.y, 1.);
            [
                RawVertex {
                    pos: corners[0],
                    texcoord: glam::vec3(0., 1., texture) * size,
                    normal,
                },
                RawVertex {
                    pos: corners[1],
                    texcoord: glam::vec3(1., 1., texture) * size,
                    normal,
                },
                RawVertex {
                    pos: corners[2],
                    texcoord: glam::vec3(1., 0., texture) * size,
                    normal,
                },
                RawVertex {
                    pos: corners[3],
                    texcoord: glam::vec3(0., 0., texture) * size,
                    normal,
                },
            ]
        }

        let quads = [
            // Bottom
            quad(
                &[x0y0z0, x1y0z0, x1y0z1, x0y0z1],
                size.xz(),
                -Vec3::unit_y(),
                textures[1] as f32,
            ),
            // Top
            quad(
                &[x0y1z0, x1y1z0, x1y1z1, x0y1z1],
                size.xz(),
                Vec3::unit_y(),
                textures[0] as f32,
            ),
            // Negative X
            quad(
                &[x0y0z0, x0y0z1, x0y1z1, x0y1z0],
                size.zy(),
                -Vec3::unit_x(),
                textures[3] as f32,
            ),
            // Positive X
            quad(
                &[x1y0z0, x1y0z1, x1y1z1, x1y1z0],
                size.zy(),
                Vec3::unit_x(),
                textures[2] as f32,
            ),
            // Negative Z
            quad(
                &[x0y0z0, x1y0z0, x1y1z0, x0y1z0],
                size.xy(),
                -Vec3::unit_z(),
                textures[5] as f32,
            ),
            // Positive Z
            quad(
                &[x0y0z1, x1y0z1, x1y1z1, x0y1z1],
                size.xy(),
                Vec3::unit_z(),
                textures[4] as f32,
            ),
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

#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct RawVertex {
    pub pos: Vec3,
    pub texcoord: Vec3,
    pub normal: Vec3,
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
    palette_index: usize,
    _bump: &'bump Bump,
) -> Box<dyn FnMut(&mut State, [usize; 3]) + 'a> {
    if model.prisms.is_empty() {
        Box::new(mesh_noop)
    } else if is_full_cube(model) {
        Box::new(move |state, pos| mesh_greedy(state, pos, palette_index, &model.prisms[0]))
    } else {
        Box::new(move |state, pos| mesh_naive(state, pos, &model.prisms))
    }
}

fn is_full_cube(model: &CompiledModel) -> bool {
    model.prisms.len() == 1
        && model.prisms[0].extent == [64, 64, 64]
        && model.prisms[0].offset == [0, 0, 0]
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

/// Mesh function which uses a greedy algorithm
/// to mesh as many blocks as possible with a single prism.
///
/// Only works on full cubes (1x1x1) for now.
fn mesh_greedy(state: &mut State, pos: [usize; 3], palette_index: usize, prism: &Prism) {
    // Extend the block in the X, then the Z, then the Y axes.
    fn index(x: usize, y: usize, z: usize) -> usize {
        y * CHUNK_DIM * CHUNK_DIM + z * CHUNK_DIM + x
    }

    let indexes = state.chunk.indexes();

    // X
    let mut x = pos[0];
    while x + 1 < 16 {
        let block = indexes.get(index(x + 1, pos[1], pos[2])).unwrap() as usize;
        if block != palette_index {
            break;
        }
        x += 1;
    }

    // Z
    let mut z = pos[2];
    while z + 1 < 16 {
        let matches = (pos[0]..=x)
            .all(|x| indexes.get(index(x, pos[1], z + 1)).unwrap() as usize == palette_index);
        if matches {
            z += 1;
        } else {
            break;
        }
    }

    // Y
    let mut y = pos[1];
    while y + 1 < 16 {
        let matches = (pos[0]..=x)
            .flat_map(|x| (pos[2]..=z).map(move |z| (x, z)))
            .all(|(x, z)| indexes.get(index(x, y + 1, z)).unwrap() as usize == palette_index);
        if matches {
            y += 1;
        } else {
            break;
        }
    }

    // Push final prism to the mesh.
    let offset = Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);
    let size = Vec3::new(
        (x - pos[0] + 1) as f32,
        (y - pos[1] + 1) as f32,
        (z - pos[2] + 1) as f32,
    );
    state.mesh.push_cube(offset, size, prism.textures);

    // Mark processed blocks as finished.
    for y in pos[1]..=y {
        for z in pos[2]..=z {
            for x in pos[0]..=x {
                state.mark_finished([x, y, z]);
            }
        }
    }
}

/// Meshes a chunk: converts a volume of blocks to a [`Mesh`].
pub(super) fn mesh<'bump>(
    models: &AHashMap<String, CompiledModel>,
    chunk: &'bump Chunk,
    bump: &'bump Bump,
) -> Mesh<'bump> {
    let mesh = Mesh {
        vertices: Vec::new_in(bump),
    };
    if chunk.palette() == [BlockId::new(blocks::Air)] {
        // Fast path: the chunk is completely air,
        // so return an empty mesh.
        return mesh;
    }

    let mut remaining = BitSet::new_in(CHUNK_VOLUME, bump);
    remaining.fill();
    let mut state = State {
        chunk,
        bump,
        mesh,
        remaining,
    };

    let mut mesh_fns = Vec::new_in(bump);
    mesh_fns.extend(
        chunk
            .palette()
            .iter()
            .copied()
            .enumerate()
            .map(|(i, block)| {
                let model = models
                    .get(block.descriptor().slug())
                    .unwrap_or_else(|| models.get("unknown").expect("missing unknown model"));
                mesh_function(model, i, bump)
            }),
    );

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
    use std::time::Instant;

    use common::{blocks, BlockId};

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

        let bump = Bump::new();
        let start = Instant::now();
        let mesh = mesh(&models, &chunk, &bump);
        println!("Took {:?}", start.elapsed());
        /*let obj = mesh.to_obj();
        fs::write("mesh.obj", obj.as_bytes()).unwrap();*/
        let _ = mesh;
    }
}
