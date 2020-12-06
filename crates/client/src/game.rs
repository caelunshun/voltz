use std::cell::{Cell, RefCell, RefMut};

use ahash::AHashSet;
use bumpalo::Bump;
use common::{event::EventBus, world::SparseZone, World};
use hecs::{DynamicBundle, Entity, EntityRef};
use protocol::{bridge::ToServer, Bridge};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use winit::{dpi::PhysicalPosition, event::VirtualKeyCode, window::Window};

use crate::{camera::Matrices, debug::DebugData, ui::UiStore};

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

    /// Time in seconds since the previous frame.
    dt: f32,

    /// The window.
    window: Window,

    /// The set of pressed keys.
    pressed_keys: AHashSet<VirtualKeyCode>,

    /// UIs to render this frame.
    ui_store: RefCell<UiStore>,

    /// The camera projection matrices.
    matrices: Matrices,

    closed: Cell<bool>,

    pub debug_data: DebugData,

    pub mouse_pos: PhysicalPosition<f64>,
}

impl Game {
    /// Creates a new game, given:
    /// * The bridge to the server.
    /// * The `EntityBuilder` containing the player's components.
    ///   These components should be derived from the login packets sent from the server.
    /// * The bump allocator.
    pub fn new(
        bridge: Bridge<ToServer>,
        player_components: impl DynamicBundle,
        window: Window,
        bump: Bump,
    ) -> Self {
        let mut ecs = hecs::World::new();
        let player = ecs.spawn(player_components);

        let main_zone = SparseZone::new();
        let world = World::new(main_zone);

        let rng = RefCell::new(Pcg64Mcg::from_entropy());

        let events = RefCell::new(EventBus::new());

        let ui_store = RefCell::new(UiStore::default());
        let pressed_keys = AHashSet::new();
        let matrices = Default::default();

        let mouse_pos = PhysicalPosition::new(0., 0.);

        Self {
            ecs,
            player,
            world,
            events,
            bump,
            rng,
            bridge,
            dt: 0.,
            window,
            pressed_keys,
            ui_store,
            matrices,
            closed: Cell::new(false),
            debug_data: Default::default(),
            mouse_pos,
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

    pub fn bump_mut(&mut self) -> &mut Bump {
        &mut self.bump
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

    /// Gets the bridge for sending packets to the server.
    pub fn bridge(&self) -> &Bridge<ToServer> {
        &self.bridge
    }

    /// Gets the number of seconds since the previous frame.
    pub fn dt(&self) -> f32 {
        self.dt
    }

    pub fn set_dt(&mut self, dt: f32) {
        self.dt = dt;
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }

    pub fn insert_pressed_key(&mut self, key: VirtualKeyCode) {
        self.pressed_keys.insert(key);
    }

    pub fn remove_pressed_key(&mut self, key: VirtualKeyCode) {
        self.pressed_keys.remove(&key);
    }

    pub fn is_key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn ui_store(&self) -> RefMut<UiStore> {
        self.ui_store.borrow_mut()
    }

    pub fn matrices(&self) -> Matrices {
        self.matrices
    }

    pub fn set_matrices(&mut self, matrices: Matrices) {
        self.matrices = matrices;
    }

    pub fn close(&self) {
        self.closed.set(true);
    }

    pub fn should_close(&self) -> bool {
        self.closed.get()
    }
}
