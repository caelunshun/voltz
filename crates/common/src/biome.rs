/// A biome. Defines the overall look of an area of the world.
///
/// Biomes are defined by a set of properties stored in this struct.
/// Most biomes are global constants; see [`Biome::Plains`] et al.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Biome {
    slug: &'static str,
    display_name: &'static str,
}

#[allow(non_upper_case_globals)]
impl Biome {
    // Biome constants.
    pub const Ocean: &'static Biome = &Biome::new("ocean", "Ocean");
    pub const Plains: &'static Biome = &Biome::new("plains", "Plains");

    const fn new(slug: &'static str, display_name: &'static str) -> Self {
        Self { slug, display_name }
    }

    pub fn slug(&self) -> &str {
        self.slug
    }

    pub fn display_name(&self) -> &str {
        self.display_name
    }
}
