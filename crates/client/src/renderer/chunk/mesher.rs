use std::{ops::Deref, sync::Arc};

use ahash::AHashMap;

use crate::asset::{model::YamlModel, Asset, Assets};

use self::compile::CompiledModel;

mod algo;
mod compile;

/// Meshes a chunk, i.e. transforms a volume of blocks into
/// an optimized mesh with vertices and texture coordinates.
/// This works using a variant of the greedy meshing algorithm.
///
/// Meshing is offloaded to the Rayon thread pool to increase throughput.
/// Request that a chunk be meshed via `spawn()`, and poll for completed
/// meshing tasks using `poll()`.
///
/// This struct stores immutable state internally: it contains the compiled
/// block models.
#[derive(Debug)]
pub struct ChunkMesher(Arc<Mesher>);

impl ChunkMesher {
    /// Creates a new [`ChunkMesher`] from the given [`Assets`] source.
    pub fn new(
        assets: &Assets,
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

        Ok(ChunkMesher(Arc::new(Mesher { models })))
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
}
