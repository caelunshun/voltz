use std::panic::Location;

use glam::Vec2;
use stretch::{
    geometry::Size,
    style::{Dimension, Style},
};
use utils::Color;

use crate::{canvas::Paint, Path, WidgetData, WidgetState};

/// A bare, solid-color rectangle.
#[derive(Debug)]
pub struct Rectangle {
    size: Vec2,
    color: Color,
    location: &'static Location<'static>,
}

impl Rectangle {
    #[track_caller]
    pub fn new(size: Vec2, color: Color) -> Self {
        Self {
            size,
            color,
            location: Location::caller(),
        }
    }
}

impl WidgetData for Rectangle {
    type State = State;

    fn location(&self) -> &'static std::panic::Location<'static> {
        self.location
    }

    fn into_state(self) -> Self::State {
        State {
            size: self.size,
            color: self.color,
        }
    }

    fn apply_changes(
        &self,
        state: &Self::State,
        changes: &mut crate::widget::ChangeList<Self::State>,
    ) {
        let _ = (state, changes);
    }
}

#[derive(Debug)]
pub struct State {
    size: Vec2,
    color: Color,
}

impl WidgetState for State {
    fn style(&self) -> Style {
        Style {
            size: Size {
                width: Dimension::Points(self.size.x),
                height: Dimension::Points(self.size.y),
            },
            ..Default::default()
        }
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn compute_size(&mut self, _max_width: Option<f32>, _max_height: Option<f32>) -> Vec2 {
        self.size
    }

    fn draw(&mut self, bounds: utils::Rect, cv: &mut crate::Canvas) {
        cv.fill_path(&Path::rect(bounds), &Paint::new().shade_solid(self.color));
    }
}
