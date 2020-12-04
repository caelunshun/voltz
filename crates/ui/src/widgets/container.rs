use std::panic::Location;

use stretch::style::Style;

use crate::{WidgetData, WidgetState};

/// A container widget, used to lay out its
/// children.
#[derive(Debug)]
pub struct Container {
    style: Style,
    location: &'static Location<'static>,
}

impl Container {
    /// Creates a new `Container` that will lay out
    /// its children in a column.
    #[track_caller]
    pub fn column() -> Self {
        Self {
            style: Style {
                flex_direction: stretch::style::FlexDirection::Column,
                ..Default::default()
            },
            location: Location::caller(),
        }
    }

    /// Updates the style.
    pub fn with_style(mut self, style: impl FnOnce(&mut Style)) -> Self {
        style(&mut self.style);
        self
    }
}

impl WidgetData for Container {
    type State = Self;

    fn location(&self) -> &'static std::panic::Location<'static> {
        self.location
    }

    fn into_state(self) -> Self::State {
        self
    }

    fn apply_changes(
        &self,
        state: &Self::State,
        changes: &mut crate::widget::ChangeList<Self::State>,
    ) {
        let _ = (state, changes);
    }
}

impl WidgetState for Container {
    fn style(&self) -> Style {
        self.style
    }

    fn draw(&mut self, bounds: utils::Rect, cv: &mut crate::Canvas) {
        let _ = (bounds, cv);
    }
}
