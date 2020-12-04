use std::panic::Location;

use crate::Canvas;
use glam::Vec2;
use stretch::style::Style;
use utils::Rect;

pub struct ChangeList<W> {
    changes: Vec<Box<dyn FnOnce(&mut W)>>,
}

impl<W> ChangeList<W> {
    pub fn apply(&mut self, change: impl FnOnce(&mut W) + 'static) {
        self.changes.push(Box::new(change));
    }
}

pub trait WidgetData {
    type State: WidgetState;

    fn location(&self) -> &'static Location<'static>;

    fn into_state(self) -> Self::State;

    fn apply_changes(&self, state: &Self::State, changes: &mut ChangeList<Self::State>);
}

pub trait WidgetState {
    fn style(&self) -> Style;

    fn is_leaf(&self) -> bool {
        false
    }

    fn compute_size(&self) -> Vec2 {
        Vec2::zero()
    }

    fn draw(&mut self, bounds: Rect, cv: &mut Canvas);
}
