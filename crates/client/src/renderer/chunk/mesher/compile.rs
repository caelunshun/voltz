use std::borrow::Cow;

use ahash::AHashMap;
use anyhow::{anyhow, Context};

use crate::asset::model::YamlModel;

/// A model which has been compiled from its high-level representation
/// to an optimized format used by the mesher. Notably, this
/// compiled format does not include inheritance.
///
/// All units are measured in stops of 1/64 block.
pub struct CompiledModel {
    /// The rectangular prisms composing this model.
    pub prisms: Vec<Prism>,
}

pub struct Prism {
    /// Offset in stops from the block origin of the minimum coordinate.
    pub offset: [u8; 3],
    /// Size in stops along each axis.
    pub extent: [u8; 3],
    /// The texture index to use for each face.
    /// Order is [top, bottom, posx, negx, posz, negz]
    pub textures: [u32; 6],
}

/// Compiler state to convert `YamlModel`s to `CompiledModel`s.
struct Compiler;

impl Compiler {
    pub fn new() -> Self {
        Self
    }

    pub fn compile(
        &mut self,
        name: &str,
        get_model: &impl Fn(&str) -> Option<YamlModel>,
        get_texture_index: &impl Fn(&str) -> Option<u32>,
    ) -> anyhow::Result<CompiledModel> {
        let model = get_model(name).ok_or_else(|| anyhow!("missing model '{}'", name))?;
        let model = self
            .make_inherited(name, &model, get_model)
            .with_context(|| format!("failed to apply inheritance for model '{}'", name))?;

        // Build up the compiled model.
        let mut prisms = Vec::new();
        for prism in &model.prisms {
            // Determine the textures used for each face.
            let mut textures = [0u32; 6];
            for (i, face) in prism.faces.iter().enumerate() {
                let texture_param = &face.texture;
                let texture_name = Self::determine_texture(&model, texture_param)?;
                let texture = get_texture_index(texture_name)
                    .ok_or_else(|| anyhow!("missing texture '{}'", texture_name))?;
                textures[i] = texture;
            }

            let prism = Prism {
                offset: prism.offset.into(),
                extent: prism.extent.into(),
                textures,
            };
            prisms.push(prism);
        }

        todo!()
    }

    fn determine_texture<'b>(model: &'b YamlModel, texture_param: &str) -> anyhow::Result<&'b str> {
        // Determine the texture to use:
        // * If the model's textures contains the parameter, use that texture.
        // * Otherwise, default to the default value for this texture argument.
        model
            .textures
            .get(texture_param)
            .or_else(|| {
                model
                    .texture_parameters
                    .get(texture_param)
                    .map(|param| param.default.as_ref())
                    .flatten()
            })
            .ok_or_else(|| {
                anyhow!(
                    "could not determine texture to use for texture parameter '{}'",
                    texture_param
                )
            })
            .map(String::as_str)
    }

    fn make_inherited<'b>(
        &mut self,
        name: &str,
        model: &'b YamlModel,
        get_model: &impl Fn(&str) -> Option<YamlModel>,
    ) -> anyhow::Result<Cow<'b, YamlModel>> {
        if let Some(parent) = &model.inherits {
            let parent_model = get_model(&parent).ok_or_else(|| {
                anyhow!("missing parent model '{}' (for child '{}')", parent, name)
            })?;
            let parent = self.make_inherited(parent, &parent_model, get_model)?;
            let mut model = model.clone();

            // Merge texture parameters
            model
                .texture_parameters
                .extend(parent.texture_parameters.clone());

            // Merge prisms
            model.prisms.extend(parent.prisms.iter().cloned());

            // Merge textures
            model.textures.extend(parent.textures.clone());

            Ok(Cow::Owned(model))
        } else {
            Ok(Cow::Borrowed(model))
        }
    }
}

/// Compiles a list of `YamlModel`s to a mapping of `CompiledModel`s.
pub fn compile<'a>(
    models: impl IntoIterator<Item = &'a str>,
    get_model: impl Fn(&str) -> Option<YamlModel>,
    get_texture_index: impl Fn(&str) -> Option<u32>,
) -> anyhow::Result<AHashMap<String, CompiledModel>> {
    let mut result = AHashMap::new();

    let mut compiler = Compiler::new();

    for model in models {
        let compiled = compiler
            .compile(model, &get_model, &get_texture_index)
            .with_context(|| format!("failed to compile model '{}'", model))?;
        result.insert(model.to_owned(), compiled);
    }

    Ok(result)
}
