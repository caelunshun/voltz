use std::{path::Path, sync::atomic::AtomicU64};

use crate::{Canvas, WidgetData, WidgetState};
use ahash::AHashMap;
use glam::vec2;
use stretch::{
    geometry::Size,
    node::Node,
    number::Number,
    style::{Dimension, Style},
    Stretch,
};
use utils::{Color, Rect};

/// The unique ID of a UI node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(u64);

impl NodeId {
    pub(crate) fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

/// Stores the persistent node tree
/// as well as the UI canvas.
pub struct Ui {
    canvas: Canvas,

    stretch: Stretch,
    root_stretch_node: Node,

    tree: Tree,
}

impl Ui {
    /// Creates a new `Ui` to render to the given
    /// pixel width and height.
    ///
    /// # Panics
    /// Panics if either pixel dimensions is zero.
    pub fn new(pixel_width: u32, pixel_height: u32, scale: f32) -> Self {
        let canvas = Canvas::new(pixel_width, pixel_height, scale);

        let mut stretch = Stretch::new();
        let root_stretch_node = stretch
            .new_node(
                Style {
                    size: Size {
                        width: Dimension::Points(canvas.width()),
                        height: Dimension::Points(canvas.height()),
                    },
                    ..Default::default()
                },
                Vec::new(),
            )
            .unwrap();

        let tree = Tree::default();

        Self {
            canvas,
            stretch,
            root_stretch_node,
            tree,
        }
    }

    /// Returns a `UiBuilder` to build the UI. New widgets
    /// are added to the UI, widgets from the previous
    /// `build()` call are persited, and missing widgets are removed.
    pub fn build(&mut self) -> UiBuilder {
        UiBuilder {
            ui: self,
            current_parent: None,
        }
    }

    /// Renders to the canvas.
    pub fn render(&mut self) {
        self.compute_layout();
        self.canvas.clear(Color::rgb(0., 0., 0.));
        let Self {
            canvas, stretch, ..
        } = self;
        self.tree.traverse(|_id, slot| {
            let layout = stretch.layout(slot.stretch_node).unwrap();
            let bounds = Rect {
                pos: vec2(layout.location.x, layout.location.y),
                size: vec2(layout.size.width, layout.size.height),
            };
            slot.node.draw(bounds, canvas);
        });
    }

    /// Gets the rendered pixel data as RGBA.
    pub fn data(&self) -> &[u8] {
        self.canvas.data()
    }

    /// Saves the rendered canvas as a PNG. Only
    /// used for testing.
    ///
    /// # Panics
    /// Panics if an IO error occurs.
    pub fn save_png(&self, path: impl AsRef<Path>) {
        self.canvas.save_png(path.as_ref())
    }

    fn compute_layout(&mut self) {
        self.stretch
            .compute_layout(
                self.root_stretch_node,
                Size {
                    width: Number::Defined(self.canvas.width()),
                    height: Number::Defined(self.canvas.height()),
                },
            )
            .unwrap();
    }

    fn insert_node(&mut self, parent: Option<NodeId>, node: Box<dyn WidgetState>) {
        let stretch_node = self.create_stretch_node(&*node);
        let slot = NodeSlot { node, stretch_node };
        let id = NodeId::next();
        self.tree.nodes.insert(id, slot);
        if let Some(parent) = parent {
            self.tree.children.entry(parent).or_default().push(id);
        } else {
            self.tree.roots.push(id);
        }

        let stretch_parent = match parent {
            Some(p) => self.tree.nodes[&p].stretch_node,
            None => self.root_stretch_node,
        };
        self.stretch
            .add_child(stretch_parent, stretch_node)
            .unwrap();
    }

    fn create_stretch_node(&mut self, node: &dyn WidgetState) -> Node {
        if node.is_leaf() {
            let computed_size = node.compute_size();
            let measure = Box::new(move |constraints: stretch::geometry::Size<Number>| {
                let mut size = stretch::geometry::Size {
                    width: computed_size.x,
                    height: computed_size.y,
                };
                // Fit to aspect ratio.
                match (constraints.width, constraints.height) {
                    (Number::Undefined, Number::Undefined) => (),
                    (Number::Defined(width), Number::Undefined) => {
                        size.height = width * size.height / size.width;
                        size.width = width;
                    }
                    (Number::Undefined, Number::Defined(height)) => {
                        size.width = height * size.width / size.height;
                        size.height = height;
                    }
                    (Number::Defined(width), Number::Defined(height)) => {
                        size.width = width;
                        size.height = height;
                    }
                }
                Ok(size)
            });
            self.stretch.new_leaf(node.style(), measure).unwrap()
        } else {
            self.stretch.new_node(node.style(), Vec::new()).unwrap()
        }
    }
}

/// Builder to add nodes to a UI while diffing.
pub struct UiBuilder<'a> {
    ui: &'a mut Ui,
    current_parent: Option<NodeId>,
}

impl<'a> UiBuilder<'a> {
    /// Pushes a new child node to the current parent.
    pub fn push<D>(&mut self, data: D) -> &mut Self
    where
        D: WidgetData,
        D::State: WidgetState + 'static,
    {
        let node = data.into_state();
        self.ui.insert_node(self.current_parent, Box::new(node));
        self
    }
}

struct NodeSlot {
    node: Box<dyn WidgetState>,
    stretch_node: Node,
}

#[derive(Default)]
struct Tree {
    nodes: AHashMap<NodeId, NodeSlot>,
    children: AHashMap<NodeId, Vec<NodeId>>,
    roots: Vec<NodeId>,
}

impl Tree {
    /// Performs a depth-first traversal of the node tree.
    pub fn traverse(&mut self, mut callback: impl FnMut(NodeId, &mut NodeSlot)) {
        let mut stack = self.roots.clone();
        while let Some(id) = stack.pop() {
            let slot = self.nodes.get_mut(&id).unwrap();
            callback(id, slot);
            stack.extend_from_slice(
                self.children
                    .get(&id)
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            );
        }
    }
}
