use std::cell::{RefCell, RefMut};

use bumpalo::Bump;
use common::{entity::player::PlayerBundle, event::EventBus, world::SparseZone, World};
use hecs::{Entity, EntityRef};
use protocol::{bridge::ToServer, Bridge};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;

/// Uberstruct containing the game state. Includes zones, entities,
/// blocks, etc.
///
/// Game state in the `client` `Game` struct is limited to the client's knowledge.
/// Chunks and entities outside of the view distance are not known to the client.
pub struct Game {
    /// The entity-component container, which contains all entities.
    ecs: hecs::World,

    /// The player using this client.
    player: Entity,

    /// All zones, chunks, and blocks in the world.
    ///
    /// This does not contain entities or block entities.
    world: World<SparseZone>,

    /// Event bus.
    events: RefCell<EventBus>,

    /// Bump allocator for the main thread.
    /// Reset each tick.
    bump: Bump,

    /// General-purpose non-cryptographic RNG.
    rng: RefCell<Pcg64Mcg>,

    /// Connection with the server.
    bridge: Bridge<ToServer>,
}

impl Game {
    /// Creates a new game, given:
    /// * The bridge to the server.
    /// * The `EntityBuilder` containing the player's components.
    ///   These components should be derived from the login packets sent from the server.
    /// * The bump allocator.
    pub fn new(bridge: Bridge<ToServer>, player_components: PlayerBundle, bump: Bump) -> Self {
        let mut ecs = hecs::World::new();
        let player = ecs.spawn(player_components);

        let main_zone = SparseZone::new();
        let world = World::new(main_zone);

        let rng = RefCell::new(Pcg64Mcg::from_entropy());

        let events = RefCell::new(EventBus::new());

        Self {
            ecs,
            player,
            world,
            events,
            bump,
            rng,
            bridge,
        }
    }

    /// Gets the entity-component container.
    pub fn ecs(&self) -> &hecs::World {
        &self.ecs
    }

    /// Mutably gets the entity-component container.
    pub fn ecs_mut(&mut self) -> &mut hecs::World {
        &mut self.ecs
    }

    /// Gets the player using this client.
    ///
    /// It is illegal to remove the returned `Entity` from the ECS.
    pub fn player(&self) -> Entity {
        self.player
    }

    /// Gets an [`EntityRef`](hecs::EntityRef) for the player using this client.
    pub fn player_ref(&self) -> EntityRef {
        self.ecs.entity(self.player).expect("player despawned")
    }

    /// Gets the event bus for queuing and processing events.
    pub fn events(&self) -> RefMut<EventBus> {
        self.events.borrow_mut()
    }

    /// Gets the bump allocator. Use this allocator for temporary
    /// allocations in hot code.
    pub fn bump(&self) -> &Bump {
        &self.bump
    }

    /// Gets the non-cryptographic random number generator used
    /// by the game.
    pub fn rng(&self) -> RefMut<impl Rng> {
        self.rng.borrow_mut()
    }

    /// Gets the [`World`](common::World) containing zones, chunks, and blocks
    /// (but not block entities).
    pub fn world(&self) -> &World<SparseZone> {
        &self.world
    }

    /// Mutably gets the [`World`](common::World) containing zones, chunks and blocks
    /// (but not block entities).
    pub fn world_mut(&mut self) -> &mut World<SparseZone> {
        &mut self.world
    }

    /// Convenience function to get the main zone.
    pub fn main_zone(&self) -> &SparseZone {
        self.world().main_zone()
    }

    /// Convenience function to mutably get the main zone.
    pub fn main_zone_mut(&mut self) -> &mut SparseZone {
        self.world_mut().main_zone_mut()
    }
}
