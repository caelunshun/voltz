use std::{iter, ops::Deref, sync::Arc};

use ahash::AHashMap;
use common::{Chunk, ChunkPos};
use crossbeam_queue::SegQueue;
use wgpu::util::DeviceExt;

use crate::{
    asset::{model::YamlModel, Asset, Assets},
    renderer::Resources,
};

use self::compile::CompiledModel;

mod algo;
mod compile;

/// A mesh uploaded to the GPU.
#[derive(Debug)]
pub struct GpuMesh {
    pub vertex_buffer: wgpu::Buffer,
}

/// Meshes a chunk, i.e. transforms a volume of blocks into
/// an optimized mesh with vertices and texture coordinates.
/// This works using a variant of the greedy meshing algorithm.
///
/// Meshing is offloaded to the Rayon thread pool to increase throughput.
/// Request that a chunk be meshed via `spawn()`, and poll for completed
/// meshing tasks using `iter_finished()`.
///
/// This struct stores immutable state internally: it contains the compiled
/// block models.
#[derive(Debug)]
pub struct ChunkMesher(Arc<Mesher>);

impl ChunkMesher {
    /// Creates a new [`ChunkMesher`] from the given [`Assets`] source.
    pub fn new(
        assets: &Assets,
        resources: &Arc<Resources>,
        get_texture_index: impl Fn(&str) -> Option<u32>,
    ) -> anyhow::Result<Self> {
        let prefix = "model/block/";

        let models: AHashMap<String, Asset<YamlModel>> = assets
            .iter_prefixed::<YamlModel>(prefix)
            .map(|(name, model)| {
                (
                    name.strip_prefix(prefix)
                        .expect("prefix")
                        .strip_suffix(".yml")
                        .expect("suffix")
                        .to_owned(),
                    model,
                )
            })
            .collect();

        let models = compile::compile(
            models.keys().map(String::as_str),
            |model| models.get(model).map(Asset::deref).map(YamlModel::clone),
            get_texture_index,
        )?;

        Ok(ChunkMesher(Arc::new(Mesher {
            models,
            resources: Arc::clone(resources),
            completed: SegQueue::new(),
        })))
    }

    /// Spawns a meshing task. The generated mesh will be
    /// returned from [`iter_finished`] at some point in the future.
    pub fn spawn(&self, pos: ChunkPos, chunk: Chunk) {
        let mesher = Arc::clone(&self.0);
        rayon::spawn(move || {
            utils::THREAD_BUMP.with(|bump| {
                let mut bump = bump.borrow_mut();
                {
                    let mesh = algo::mesh(&mesher.models, &chunk, &bump);
                    let label = format!("chunk_mesh_{:?}", pos);
                    let mesh = mesher.upload(&label, &mesh);

                    mesher.completed.push((pos, mesh));
                }
                bump.reset();
            });
        });
    }

    /// Returns an iterator over meshes which have completed.
    pub fn iter_finished<'a>(&'a self) -> impl Iterator<Item = (ChunkPos, GpuMesh)> + 'a {
        iter::from_fn(move || self.0.completed.pop())
    }
}

#[derive(Debug)]
struct Mesher {
    /// The compiled block models. This maps block slug
    /// to its model.
    ///
    /// A block which has no entry here should defer to
    /// the entry called "unknown."
    models: AHashMap<String, CompiledModel>,

    resources: Arc<Resources>,

    /// Completed meshes.
    completed: SegQueue<(ChunkPos, GpuMesh)>,
}

impl Mesher {
    pub fn upload(&self, label: &str, mesh: &algo::Mesh) -> GpuMesh {
        let vertices: &[u8] = bytemuck::cast_slice(mesh.vertices.as_slice());
        let vertex_buffer =
            self.resources
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: vertices,
                    usage: wgpu::BufferUsage::VERTEX,
                });

        GpuMesh { vertex_buffer }
    }
}
