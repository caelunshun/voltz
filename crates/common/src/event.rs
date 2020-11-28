use std::{
    any::{Any, TypeId},
    collections::VecDeque,
};

use ahash::AHashMap;

/// An _event bus_ for queuing and processing events.
///
/// Events can be of any 'static type.
///
/// Events are processed in a _polling_ fashion: each system
/// polls the event bus for new events when it runs. Events
/// which have been observed by each system are dropped.
///
/// This has the consequence of _events not being handled as
/// soon as `push()` is called_. If events require immediate handling
/// so that handler side effects are observed, then normal method
/// calls are better suited.
///
/// # System indexing
/// The event bus internally stores events in the order they
/// were added. Each event is associated with the _system index_
/// it was invoked by. When that system runs again, the bus assumes
/// all other systems have observed those events are therefore drops them.
#[derive(Default)]
pub struct EventBus {
    seats: AHashMap<TypeId, Box<dyn ErasedSeat>>,
    system: usize,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_system(&mut self, system: usize) {
        self.system = system;
        for seat in self.seats.values_mut() {
            seat.advance_to(system);
        }
    }

    pub fn push<T>(&mut self, event: T)
    where
        T: 'static,
    {
        let system = self.system;
        let seat = self.seat::<T>();
        seat.push(event, system);
    }

    pub fn iter<'a, T>(&'a mut self) -> impl Iterator<Item = &'a T> + 'a
    where
        T: 'static,
    {
        let seat = self.seat::<T>();
        seat.iter()
    }

    fn seat<T>(&mut self) -> &mut Seat<T>
    where
        T: 'static,
    {
        self.seats
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(Seat::<T>::default()))
            .as_any_mut()
            .downcast_mut()
            .expect("mismatched types")
    }
}

trait ErasedSeat {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn advance_to(&mut self, system_index: usize);
}

struct Slot<T> {
    event: T,
    system: usize,
}

struct Seat<T> {
    events: VecDeque<Slot<T>>,
}

impl<T> Default for Seat<T> {
    fn default() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }
}

impl<T> Seat<T> {
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T> + 'a {
        self.events.iter().map(|slot| &slot.event)
    }

    pub fn push(&mut self, event: T, system: usize) {
        self.events.push_back(Slot { event, system });
    }
}

impl<T> ErasedSeat for Seat<T>
where
    T: 'static,
{
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn advance_to(&mut self, system_index: usize) {
        while let Some(event) = self.events.get(0) {
            if event.system == system_index {
                self.events.pop_front();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let mut bus = EventBus::new();

        for x in 0..100 {
            bus.push(x);
        }

        assert_eq!(
            bus.iter::<i32>().copied().collect::<Vec<_>>(),
            (0..100).collect::<Vec<_>>()
        );
    }

    #[test]
    fn dropping() {
        let mut bus = EventBus::new();

        for x in 0..100 {
            bus.push(x);
        }

        bus.set_system(1);

        assert_eq!(
            bus.iter::<i32>().copied().collect::<Vec<_>>(),
            (0..100).collect::<Vec<_>>()
        );

        bus.set_system(0);
        assert_eq!(bus.iter::<i32>().count(), 0);

        bus.set_system(1);
        assert_eq!(bus.iter::<i32>().count(), 0);
    }
}
