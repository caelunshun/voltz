use std::cell::{RefCell, RefMut};

use bumpalo::Bump;
use common::{event::EventBus, World, Zone};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;

/// Uberstruct containing the entire game state.
///
/// The server is omniscient: it knows about the entire
/// world and all entities. By contrast, a client knows
/// only what it has been told by the server.
pub struct Game {
    /// The ECS containing all entities.
    ecs: hecs::World,

    /// The world containing all zones.
    world: World<Zone>,

    /// The event bus.
    events: RefCell<EventBus>,

    /// The bump allocator.
    bump: Bump,

    /// The non-cryptographic RNG used for game operations.
    rng: RefCell<Pcg64Mcg>,
}

impl Game {
    /// Creates a new [`Game`] given the main zone.
    pub fn new(main_zone: Zone) -> Self {
        let ecs = hecs::World::new();
        let world = World::new(main_zone);
        let events = RefCell::new(EventBus::new());
        let bump = Bump::new();
        let rng = RefCell::new(Pcg64Mcg::from_entropy());

        Self {
            ecs,
            world,
            events,
            bump,
            rng,
        }
    }

    /// Gets the ECS containing entities.
    pub fn ecs(&self) -> &hecs::World {
        &self.ecs
    }

    /// Mutably gets the ECS containing entities.
    pub fn ecs_mut(&mut self) -> &mut hecs::World {
        &mut self.ecs
    }

    /// Gets the world containing zones, chunks, and blocks.
    pub fn world(&self) -> &World<Zone> {
        &self.world
    }

    /// Mutably gets the world cotaining zones, chunks, and blocks.
    pub fn world_mut(&mut self) -> &mut World<Zone> {
        &mut self.world
    }

    /// Convenience function to get the main zone.
    pub fn main_zone(&self) -> &Zone {
        self.world().main_zone()
    }

    /// Convenience function to mutably get the main zone.
    pub fn main_zone_mut(&mut self) -> &mut Zone {
        self.world_mut().main_zone_mut()
    }

    /// Gets the event bus, used to process or enqueue events.
    pub fn events(&self) -> RefMut<EventBus> {
        self.events.borrow_mut()
    }

    /// Gets the _non-cryptocraphic_ RNG for game logic.
    pub fn rng(&self) -> RefMut<impl Rng> {
        self.rng.borrow_mut()
    }

    /// Gets a bump allocator for efficient short-lived allocations.
    pub fn bump(&self) -> &Bump {
        &self.bump
    }

    pub fn bump_mut(&mut self) -> &mut Bump {
        &mut self.bump
    }
}
