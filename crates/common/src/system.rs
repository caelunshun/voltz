/// A simple system executor.
///
/// Each system is conceptually a `fn(&mut self, &mut State)`,
/// where `State` is the game state (`client::game::Game` or `server::game::Game`).
///
/// Systems run in the order they were added to the executor.
/// The order is therefore well-defined.
///
/// Unlike many ECS
/// libraries, we choose to run systems sequentially,
/// which allows a single struct to be passed to each
/// system. Whereas in some ECSs like `bevy-ecs` you end
/// up with functions taking fifteen arguments since
/// the scheduler needs to know what data is accessed to
/// parallelize. We do not consider the performance benefits
/// from parallel systems to be worth the maintenance and practical
/// cost.
pub struct SystemExecutor<S> {
    systems: Vec<Box<dyn System<S>>>,
}

impl<S> SystemExecutor<S>
where
    S: 'static,
{
    /// Creates an empty `SystemExecutor`.
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    /// Adds a system to the executor, returning `self`
    /// for method chaining.
    pub fn add(&mut self, system: impl System<S>) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    /// Runs all systems in order.
    pub fn run(&mut self, game: &mut S) {
        for system in &mut self.systems {
            system.run(game);
        }
    }
}

/// A system that can be added to a [`SystemExecutor`].
///
/// This trait is implemented for all `fn(&mut S)`s.
/// Stateful systems should use a struct implementing
/// this trait.
pub trait System<S>: 'static {
    fn run(&mut self, game: &mut S);

    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl<S, F> System<S> for F
where
    F: FnMut(&mut S) + 'static,
{
    fn run(&mut self, game: &mut S) {
        self(game)
    }
}
