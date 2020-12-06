//! UI management. Allows any number
//! of UIs to be positioned on the screen
//! and rendered.

use std::alloc::AllocRef;

use ahash::AHashMap;
use glam::Vec2;
use voltzui::Ui;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Length {
    /// Measured in logical pixels.
    LogicalPixels(f32),
    /// Measured as a percentage of the window size.
    Percent(f32),
}

impl Length {
    pub fn resolve(self, of: f32) -> f32 {
        match self {
            Length::LogicalPixels(p) => p,
            Length::Percent(p) => (p / 100.) * of,
        }
    }
}

/// Contains UIs to be rendered this frame.
///
/// A UI is created the first time `get()` is
/// called with its name. It is dropped
/// if on any given tick `get()` is not called
/// for that UI again.
#[derive(Default)]
pub struct UiStore {
    uis: AHashMap<&'static str, StoredUi>,
}

impl UiStore {
    /// Gets a UI with the given name, dimensions, and position.
    /// Position is measured in logical pixels.
    pub fn get(&mut self, name: &'static str, width: Length, height: Length, pos: Vec2) -> &mut Ui {
        let stored = self.uis.entry(name).or_insert_with(|| {
            log::debug!("Creating UI '{}'", name);
            StoredUi {
                ui: Ui::new(),
                width,
                height,
                pos,
                accessed: true,
            }
        });
        stored.accessed = true;
        stored.width = width;
        stored.height = height;
        stored.pos = pos;
        &mut stored.ui
    }

    /// Finishes the current frame, removing any UIs
    /// which were not accessed. Writes UI render data
    /// to `output`.
    pub fn finish_frame<'a, A: AllocRef>(&'a mut self, output: &mut Vec<UiRenderData<'a>, A>) {
        self.uis.retain(|&name, stored| {
            if !stored.accessed {
                log::debug!("Removing UI '{}'", name);
                return false;
            }

            stored.accessed = false;
            true
        });

        for stored in self.uis.values_mut() {
            output.push(UiRenderData {
                ui: &mut stored.ui,
                width: stored.width,
                height: stored.height,
                pos: stored.pos,
            });
        }
    }
}

/// A UI to be rendered.
pub struct UiRenderData<'a> {
    pub ui: &'a mut Ui,
    pub width: Length,
    pub height: Length,
    pub pos: Vec2,
}

struct StoredUi {
    ui: Ui,
    width: Length,
    height: Length,
    pos: Vec2,
    /// Whether the UI has been accessed this tick
    accessed: bool,
}
