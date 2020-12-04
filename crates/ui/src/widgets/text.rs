use std::{panic::Location, sync::Arc};

use fontdue::{
    layout::{HorizontalAlign, Layout, VerticalAlign},
    Font,
};
use glam::{vec2, Vec2};
use stretch::{geometry::Size, style::Dimension};

use crate::{canvas::TextSettings, WidgetData, WidgetState};

const DEFAULT_SIZE: f32 = 14.;

/// Render some text.
pub struct Text<'a> {
    text: &'a str,
    settings: TextSettings,
    location: &'static Location<'static>,
}

impl<'a> Text<'a> {
    #[track_caller]
    pub fn new(text: &'a str, font: &Arc<Font>) -> Self {
        Self {
            text,
            settings: TextSettings {
                font: Arc::clone(font),
                align_h: HorizontalAlign::Left,
                align_v: VerticalAlign::Top,
                size: DEFAULT_SIZE,
                pos: Vec2::zero(),
                max_width: None,
                max_height: None,
            },
            location: Location::caller(),
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.settings.size = size;
        self
    }

    pub fn align_h(mut self, align: HorizontalAlign) -> Self {
        self.settings.align_h = align;
        self
    }

    pub fn aligh_v(mut self, align: VerticalAlign) -> Self {
        self.settings.align_v = align;
        self
    }
}

impl WidgetData for Text<'_> {
    type State = State;

    fn location(&self) -> &'static std::panic::Location<'static> {
        self.location
    }

    fn into_state(self) -> Self::State {
        let mut layout_engine = Layout::new(fontdue::layout::CoordinateSystem::PositiveYDown);
        self.settings.layout(self.text, &mut layout_engine);
        let width = layout_engine
            .glyphs()
            .iter()
            .map(|pos| {
                (pos.x
                    + self
                        .settings
                        .font
                        .metrics(pos.key.c, self.settings.size)
                        .advance_width) as i32
            })
            .max()
            .unwrap_or_default() as f32;
        let height = layout_engine.height();
        State {
            size: vec2(width, height),
            text: self.text.to_owned(),
            settings: self.settings,
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

pub struct State {
    text: String,
    size: Vec2,
    settings: TextSettings,
}

impl WidgetState for State {
    fn style(&self) -> stretch::style::Style {
        stretch::style::Style {
            max_size: Size {
                width: Dimension::Points(self.size.x),
                height: Dimension::Points(self.size.y),
            },
            ..Default::default()
        }
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn compute_size(&self) -> Vec2 {
        self.size
    }

    fn draw(&mut self, bounds: utils::Rect, cv: &mut crate::Canvas) {
        self.settings.max_width = Some(bounds.size.x);
        self.settings.max_height = Some(bounds.size.y);
        self.settings.pos = bounds.pos;

        cv.fill_text(&self.text, &self.settings);
    }
}
