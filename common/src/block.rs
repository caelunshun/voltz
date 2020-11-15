//! Block API.

use std::any::{Any, TypeId};

use ahash::AHashMap;
use once_cell::sync::Lazy;

pub mod blocks;

/// The block registry. Aids conversion between `BlockId` and the individual
/// block structs (`Dirt`, `Stone`, etc.). Also helps access shared properties.
#[derive(Default)]
struct Registry {
    /// Maps block struct TypeId to BlockId.kind.
    type_to_kind: AHashMap<TypeId, u32>,
    /// Maps BlockId.kind to struct TypeId.
    kind_to_type: Vec<TypeId>,
    /// Maps BlockId.kind to BlockDescriptor.
    kind_to_descriptor: Vec<BlockDescriptor>,

    /// The next BlockId.kind to allocate.
    next_kind: u32,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<T: Block>(&mut self) -> &mut Self {
        self.type_to_kind.insert(TypeId::of::<T>(), self.next_kind);
        self.next_kind += 1;

        self.kind_to_type.push(TypeId::of::<T>());

        self.kind_to_descriptor.push(T::descriptor());

        self
    }

    pub fn kind_of<T: Block>(&self) -> Option<u32> {
        self.type_to_kind.get(&TypeId::of::<T>()).copied()
    }

    pub fn type_of(&self, kind: u32) -> Option<TypeId> {
        self.kind_to_type.get(kind as usize).copied()
    }

    pub fn descriptor_of(&self, kind: u32) -> Option<BlockDescriptor> {
        self.kind_to_descriptor.get(kind as usize).copied()
    }
}

/// The global block registry.
static REGISTRY: Lazy<Registry> = Lazy::new(|| {
    let mut registry = Registry::new();

    use blocks::*;
    registry
        .register::<Air>()
        .register::<Dirt>()
        .register::<Stone>()
        .register::<Grass>();

    registry
});

/// ID of a block state.
///
/// This struct can be thought of as a `Box<dyn Block>`, except
/// it provides additional utilities and is much more efficient
/// (it's just two integers with no heap allocations).
///
/// So long as the block registry is not updated, block IDs will be stable
/// across different program environments. This means that provided both
/// client and server use the same game version, we can directly serialize
/// block IDs over the network. If we do not have this version guarantee,
/// as is the case for saveload, we need to serialize the block slug and properties
/// map.
///
/// A block ID consists of two `u32`s: the block _kind_ ID,
/// which identifies which type this block is ("dirt", "chest"),
/// and the state ID, which determines the set of property
/// values for this block state (e.g. "facing: Facing::North").
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct BlockId {
    kind: u32,
    state: u32,
}

static BLOCK_INVALID: &str =
    "block has not been registered with the block registry, or its state is invalid.";

impl BlockId {
    /// Creates a `BlockId` from the provided type which implements `Block`.
    ///
    /// # Panics
    /// Panics if `T` is not registered with the block registry. This would
    /// be the case if you've implemented `Block` for an external type. In general,
    /// this will not happen.
    pub fn new<T: Block>(block: T) -> Self {
        let kind = REGISTRY.kind_of::<T>().expect(BLOCK_INVALID);
        let state = block.state_id();

        Self::from_raw_parts(kind, state)
    }

    /// Creates a block from a raw kind and state ID.
    ///
    /// # Warning
    /// It is possible to create an invalid BlockId using
    /// this method, which can result in panics (not memory unsafety).
    /// This method is intended for use in testing only.
    pub fn from_raw_parts(kind: u32, state: u32) -> Self {
        Self { kind, state }
    }

    /// Returns the descriptor of this block, which provides
    /// e.g. slug and display name.
    pub fn descriptor(self) -> BlockDescriptor {
        REGISTRY.descriptor_of(self.kind).expect(BLOCK_INVALID)
    }

    /// Attempts to get this block as a struct of type T.
    /// T must implement the `Block` trait.
    ///
    /// Use this function to downcast an arbitrary block
    /// to a concrete block type.
    pub fn cast<T: Block>(self) -> Option<T> {
        if REGISTRY.type_of(self.kind).expect(BLOCK_INVALID) == TypeId::of::<T>() {
            Some(T::from_state_id(self.state).expect(BLOCK_INVALID))
        } else {
            None
        }
    }

    /// Returns whether this block is an instance of `T`.
    /// In other words, returns whether `self.cast::<T>()` would
    /// return `Some`.
    pub fn is<T: Block>(self) -> bool {
        self.cast::<T>().is_some()
    }

    /// Returns the numeric ID of this block's kind.
    pub fn kind(self) -> u32 {
        self.kind
    }

    /// Returns the numeric ID of this block's state,
    /// which determines block property values.
    pub fn state(self) -> u32 {
        self.state
    }
}

/// Implemented by structs representing block states.
///
/// For sanity, this trait should never be implemented outside
/// of the block module.
pub trait Block: Any + Sized {
    /// Gets the state ID of this block. A future call to `from_state_id()`
    /// with the value returned from this method must create a value equal to `self`.
    fn state_id(&self) -> u32;

    /// Creates a block state from a state ID previously returned
    /// from `Self::state_id()`.
    fn from_state_id(id: u32) -> Option<Self>;

    /// Gets the BlockDescriptor for this block kind.
    fn descriptor() -> BlockDescriptor;
}

/// A descriptor that exists for every block kind. Provides
/// information such as slug and display name.
#[derive(Debug, Copy, Clone)]
pub struct BlockDescriptor {
    slug: &'static str,
    display_name: &'static str,
}

impl BlockDescriptor {
    pub fn new(slug: &'static str, display_name: &'static str) -> Self {
        Self { slug, display_name }
    }

    /// Returns the block's slug, for example "dirt." This slug is
    /// stable and can be used for serialization to disk. (The properties
    /// map of the block returned by `BlockId::to_properties()` must be serialized
    /// as well for properties to persist.)
    pub fn slug(&self) -> &str {
        self.slug
    }

    /// Returns the block's display name which can be displayed to the user.
    pub fn display_name(&self) -> &str {
        self.display_name
    }
}

/// A type which can be used as a block property.
///
/// Note that this trait is not implemented for the integer
/// types, as these are special-cased in the block proc macro.
pub trait BlockProperty: Copy {
    /// The number of possible values of this type.
    ///
    /// For enums, this is the number of variants.
    const NUM_POSSIBLE_VALUES: u32;

    /// Converts this value to an integer.
    fn to_int(self) -> u32;

    /// Gets this value from an integer.
    fn from_int(int: u32) -> Option<Self>;
}

impl BlockProperty for bool {
    const NUM_POSSIBLE_VALUES: u32 = 2;

    fn to_int(self) -> u32 {
        if self {
            1
        } else {
            0
        }
    }

    fn from_int(int: u32) -> Option<Self> {
        match int {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
}

/// A utility to map a combination of (potentially many)
/// property values to a single `u32`. This works by
/// interpreting these values in n-dimensional coordinate
/// space.
struct PropertyPacker<const AMOUNT: usize> {
    /// The stride for each property, equal to the sum
    /// of the number of possible values for each proceeding
    /// property.
    strides: [u32; AMOUNT],
}

impl<const AMOUNT: usize> PropertyPacker<AMOUNT> {
    /// Creates a new PropertyPacker. The provided array
    /// should contain the number of possible values for each
    /// property.
    pub const fn new(num_possible_values: [u32; AMOUNT]) -> Self {
        let mut strides = [0; AMOUNT];

        // Rust doesn't support for loops in const fns yet.
        let mut i = 0;
        while i < AMOUNT {
            let mut stride = 1;
            let mut j = i + 1;
            while j < AMOUNT {
                stride *= num_possible_values[j];
                j += 1;
            }

            strides[i] = stride;

            i += 1;
        }

        Self { strides }
    }

    /// Packs a sequence of property values into a single `u32`.
    ///
    /// The property value at index `i` should be within the range `[0, num_possible_values[i]]`.
    pub fn pack(&self, values: [u32; AMOUNT]) -> u32 {
        values
            .iter()
            .zip(self.strides.iter())
            .map(|(&value, &stride)| value * stride)
            .sum::<u32>()
    }

    /// Unpacks a packed `u32` into a sequence of property values.
    pub fn unpack(&self, packed: u32) -> [u32; AMOUNT] {
        let mut unpacked = [0; AMOUNT];

        let mut packed = packed;
        for (&stride, unpacked) in self.strides.iter().zip(unpacked.iter_mut()) {
            *unpacked = packed / stride;
            packed -= *unpacked * stride;
        }

        unpacked
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn property_packer_zero_size() {
        let packer = PropertyPacker::new([]);
        assert_eq!(packer.pack([]), 0);
    }

    #[test]
    fn property_packer_one_size() {
        let packer = PropertyPacker::new([2]);
        assert_eq!(packer.pack([0]), 0);
        assert_eq!(packer.pack([1]), 1);
        assert_eq!(packer.pack([2]), 2);
        assert_eq!(packer.unpack(0), [0]);
        assert_eq!(packer.unpack(1), [1]);
        assert_eq!(packer.unpack(2), [2]);
    }

    #[test]
    fn property_packer_two_size() {
        let packer = PropertyPacker::new([2, 3]);
        assert_eq!(packer.pack([0, 0]), 0);
        assert_eq!(packer.pack([1, 0]), 3);
        assert_eq!(packer.pack([1, 2]), 5);
        assert_eq!(packer.pack([0, 2]), 2);
        assert_eq!(packer.unpack(0), [0, 0]);
        assert_eq!(packer.unpack(3), [1, 0]);
        assert_eq!(packer.unpack(5), [1, 2]);
        assert_eq!(packer.unpack(2), [0, 2]);
    }

    #[test]
    fn property_packet_n_size() {
        let packer = PropertyPacker::new([5, 4, 3, 2, 1]);

        // Verify each possible value produces a unique
        // packed u32 within the correct range.
        let mut used = HashSet::new();
        let range = 0..(5 * 4 * 3 * 2 * 1);

        for a in 0..5 {
            for b in 0..4 {
                for c in 0..3 {
                    for d in 0..2 {
                        let values = [a, b, c, d, 0];
                        let packed = packer.pack(values);
                        assert_eq!(packer.unpack(packed), values);
                        assert!(used.insert(packed));
                        assert!(range.contains(&packed))
                    }
                }
            }
        }
    }

    #[test]
    fn registry_no_panic() {
        Lazy::force(&REGISTRY);
    }

    #[test]
    fn block_ids_continuous() {
        assert_eq!(BlockId::new(blocks::Air).kind(), 0);
        assert_eq!(BlockId::new(blocks::Air).state(), 0);

        assert_eq!(BlockId::new(blocks::Dirt).kind(), 1);
        assert_eq!(BlockId::new(blocks::Dirt).state(), 0);

        assert!(BlockId::from_raw_parts(0, 0).is::<blocks::Air>());
        assert!(BlockId::from_raw_parts(1, 0).is::<blocks::Dirt>());
    }
}
