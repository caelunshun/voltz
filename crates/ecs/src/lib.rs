#![allow(incomplete_features)]
#![feature(specialization)]

use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    ops::Deref,
    ops::DerefMut,
};

/// A type that may be used as a component.
///
/// Archetypes also contain `Component`s. `Component`s
/// may contain sub-components which are flattened into
/// each entity's set of components.
pub trait Component: Send + AsAny + 'static {
    /// Gets the nested component of type T from this component.
    ///
    /// T may be equal to Self, in which case this method should return `self`.
    fn nested<T>(&self) -> Option<&T>
    where
        T: Component;

    /// Same as `nested<T>` but operates on mutable references.
    fn nested_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Component;
}

default impl<C> Component for C
where
    C: Send + AsAny + 'static,
{
    default fn nested<T>(&self) -> Option<&T>
    where
        T: Component,
    {
        if TypeId::of::<C>() == TypeId::of::<T>() {
            Some(unsafe { &*(self as *const C as *const T) })
        } else {
            None
        }
    }

    default fn nested_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Component,
    {
        if TypeId::of::<C>() == TypeId::of::<T>() {
            Some(unsafe { &mut *(self as *mut C as *mut T) })
        } else {
            None
        }
    }
}

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> AsAny for T
where
    T: Any,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Unique (within one ECS) ID of an entity.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct EntityId {
    id: u32,
    version: u32,
}

#[derive(Debug, thiserror::Error)]
#[error("entity with this ID has already been removed")]
pub struct NoSuchEntity;

#[derive(Debug, thiserror::Error)]
pub enum ComponentAccessError {
    #[error("entity does not have component '{0}'")]
    NoSuchComponent(&'static str),
    #[error(transparent)]
    NoSuchEntity(NoSuchEntity),
}

/// A composition-based, data-oriented storage
/// for entities. This ECS works differently from other
/// Rust ECS libraries. Rather than a "flat" model where
/// each entity can any set of components, we require each
/// entity to belong to exactly one archetype (class). Archetypes
/// can be defined as structs with `#[derive(Archetype)]`.
///
/// This archetype approach allows us to have the benefits of ECS
/// (performance, composition) while still enjoying the ergonomics
/// and clean code found in object-oriented designs. Most object-oriented
/// constructs are, in fact, easy to model using this crate.
///
/// Inheritance can be modeled by storing a parent/superclass archetype
/// within an archetype. If this field is marked as `#[component]`, then all
/// systems and logic operating on the parent archetype will run on the child
/// as well. If this field is not marked as `#[component]`, these systems will
/// not run, but you can still leverage the methods provided by the parent class.
///
/// # Borrow checking
/// All components are borrow-checked at runtime on a fine-grained, per-entity
/// level. Furthermore, `CompRef`s are not bound to the lifetime of the `Ecs`;
/// this allows for flexibility in operations. (Internally, this works by reference-counting
/// the internal `Ecs` structure.)
#[derive(Default)]
pub struct Ecs {}

impl Ecs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an entity with the given archetype. Returns
    /// the entity's ID.
    pub fn add<A: Component>(&mut self, archetype: A) -> EntityId {
        let _ = archetype;
        todo!()
    }

    /// Removes an entity.
    pub fn remove(&mut self, entity: EntityId) -> Result<(), NoSuchEntity> {
        let _ = entity;
        todo!()
    }

    /// Gets the given component for an entity.
    pub fn get<C: Component>(&self, entity: EntityId) -> Result<CompRef<C>, ComponentAccessError> {
        let _ = entity;
        todo!()
    }

    /// Mutably gets the given component for an entity.
    pub fn get_mut<C: Component>(
        &self,
        entity: EntityId,
    ) -> Result<CompMut<C>, ComponentAccessError> {
        let _ = entity;
        todo!()
    }

    /// Iterates over all entities with the given component, yielding
    /// the components and the entity IDs.
    pub fn query<C: Component>(&self) -> impl Iterator<Item = (EntityId, CompRef<C>)> {
        std::iter::empty()
    }

    /// Same as `query()` but component references are mutable.
    pub fn query_mut<C: Component>(&self) -> impl Iterator<Item = (EntityId, CompMut<C>)> {
        std::iter::empty()
    }
}

/// A reference-counted, runtime borrow-checked handle to a component within an `Ecs.`.
pub struct CompRef<T> {
    _todo: PhantomData<T>,
}

impl<T> Deref for CompRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}

pub struct CompMut<T> {
    _todo: PhantomData<T>,
}

impl<T> Deref for CompMut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}

impl<T> DerefMut for CompMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        todo!()
    }
}
