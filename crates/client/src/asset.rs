use std::{
    any::type_name, any::type_name_of_val, any::Any, collections::HashMap, fs, marker::PhantomData,
    ops::Deref, path::Path, sync::Arc,
};

use ahash::AHashMap;
use anyhow::{anyhow, Context};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use walkdir::WalkDir;

pub mod model;

pub trait AssetKind: Any + Send + Sync {}
impl<T> AssetKind for T where T: Any + Send + Sync {}

pub trait AssetLoader: Send + Sync + 'static {
    fn load(&self, data: &[u8]) -> anyhow::Result<Box<dyn Any + Send + Sync>>;
}

type DynAsset = Arc<dyn Any + Send + Sync>;

/// A reference-counted handle to an asset of type `T`.
#[derive(Debug, Clone)]
pub struct Asset<T>(Arc<T>);

impl<T> Deref for Asset<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Asset<T> {
    fn from_dyn(asset: DynAsset) -> Option<Self>
    where
        T: AssetKind,
    {
        let asset = Arc::downcast::<T>(asset).ok()?;
        Some(Self(asset))
    }
}

/// The asset index file `index.yml`. Specifies which loader
/// to use on a per-directory basis.
#[derive(Debug, Serialize, Deserialize)]
pub struct AssetIndex {
    /// Maps directory path relative to the asset root to
    /// the name of the loader used for files within this directory.
    pub groups: HashMap<String, Group>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Group {
    pub loader: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AssetGetError {
    #[error("asset '{path}' not found")]
    Missing { path: String },
    #[error("asset '{path}' expected to be of type '{expected}'; was type '{actual}'")]
    TypeMismatch {
        path: String,
        expected: String,
        actual: String,
    },
}

#[derive(Default)]
pub struct Assets {
    assets: AHashMap<String, DynAsset>,
    loaders: AHashMap<String, Box<dyn AssetLoader>>,
}

impl Assets {
    /// Creates a new, empty `Assets`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new loader with this `Assets`.
    pub fn add_loader(&mut self, name: impl Into<String>, loader: impl AssetLoader) -> &mut Self {
        self.loaders.insert(name.into(), Box::new(loader));
        self
    }

    /// Recursively loads all assets from the given directory.
    ///
    /// The directory must contain `index.yml` which satisfies
    /// the [`AssetIndex`] format. This file specifies which loader
    /// to use for each file.
    pub fn load_dir(&mut self, directory: impl AsRef<Path>) -> anyhow::Result<()> {
        let directory = directory.as_ref();
        let index = Self::load_index(directory)?;
        self.load_assets(directory, &index)?;
        Ok(())
    }

    fn load_index(directory: &Path) -> anyhow::Result<AssetIndex> {
        let path = directory.join("index.yml");
        let bytes = fs::read(&path)?;
        let index: AssetIndex = serde_yaml::from_slice(&bytes)?;
        Ok(index)
    }

    fn load_assets(&mut self, directory: &Path, index: &AssetIndex) -> anyhow::Result<()> {
        for (subdir, group) in &index.groups {
            self.load_group(directory, Path::new(subdir), &group.loader)
                .with_context(|| {
                    format!("failed to load asset directory '{}'", directory.display())
                })?;
        }
        Ok(())
    }

    fn find_loader(&self, name: &str) -> anyhow::Result<&dyn AssetLoader> {
        self.loaders
            .get(name)
            .ok_or_else(|| anyhow!("missing asset loader '{}'", name))
            .map(|b| b.deref())
    }

    fn insert_asset(&mut self, path: &str, asset: DynAsset) {
        self.assets.insert(path.to_owned(), asset);
        log::info!("Loaded {}", path);
    }

    fn load_group(&mut self, directory: &Path, subdir: &Path, loader: &str) -> anyhow::Result<()> {
        let loader = self.find_loader(loader)?;

        let mut assets = Vec::new();
        let subdir = directory.join(subdir);
        for entry in WalkDir::new(&subdir) {
            let entry = entry?;

            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let bytes = fs::read(path)?;
            let asset = loader
                .load(&bytes)
                .with_context(|| format!("failed to load '{}'", path.display()))?;
            assets.push((path.to_path_buf(), asset));
        }

        for (path, asset) in assets {
            self.insert_asset(&path.to_string_lossy(), asset.into());
        }

        Ok(())
    }

    /// Gets the asset with the given path (relative to the asset directory)
    /// as a handle of type `T`. Returns an error if the asset does not exist
    /// or if its type is not `T`.
    pub fn get<T: AssetKind>(&self, path: &str) -> Result<Asset<T>, AssetGetError> {
        let dynamic = self
            .assets
            .get(path)
            .ok_or_else(|| AssetGetError::Missing {
                path: path.to_owned(),
            })?;

        let asset = Asset::<T>::from_dyn(Arc::clone(dynamic)).ok_or_else(|| {
            AssetGetError::TypeMismatch {
                path: path.to_owned(),
                expected: type_name::<T>().to_owned(),
                actual: type_name_of_val(dynamic).to_owned(), // TODO: this is the Arc type and not the inner type
            }
        })?;

        Ok(asset)
    }

    /// Iterates over all assets matching the given prefix and type `T`.
    pub fn iter_prefixed<'a, T: AssetKind>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = (&'a str, Asset<T>)> + 'a {
        self.assets
            .iter()
            .filter(move |(name, _)| name.starts_with(prefix))
            .filter_map(|(name, asset)| {
                let asset = Asset::from_dyn(Arc::clone(asset))?;
                Some((name.as_str(), asset))
            })
    }
}

/// Asset loader for YAML files with format `T`.
pub struct YamlLoader<T> {
    _marker: PhantomData<T>,
}

impl<T> Default for YamlLoader<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> YamlLoader<T> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T: DeserializeOwned + Any + Send + Sync> AssetLoader for YamlLoader<T> {
    fn load(&self, data: &[u8]) -> anyhow::Result<Box<dyn Any + Send + Sync>> {
        let asset: T = serde_yaml::from_slice(data)?;
        Ok(Box::new(asset))
    }
}
