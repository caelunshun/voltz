use std::{cell::RefCell, rc::Rc, sync::atomic::AtomicU64};

use crate::{Canvas, WidgetData, WidgetState};
use ahash::AHashMap;
use glam::{vec2, Vec2};
use stretch::{
    geometry::Size,
    node::Node,
    number::Number,
    style::{Dimension, Style},
    Stretch,
};
use utils::Rect;

/// The unique ID of a UI node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(u64);

impl NodeId {
    pub(crate) fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

/// Stores the persistent node tree.
pub struct Ui {
    stretch: Stretch,
    root_stretch_node: Node,

    tree: Tree,
}

impl Ui {
    /// Creates a new `Ui`.
    pub fn new() -> Self {
        let mut stretch = Stretch::new();
        let root_stretch_node = stretch
            .new_node(
                Style {
                    size: Size {
                        width: Dimension::Percent(1.),
                        height: Dimension::Percent(1.),
                    },
                    ..Default::default()
                },
                Vec::new(),
            )
            .unwrap();

        let tree = Tree::default();

        Self {
            stretch,
            root_stretch_node,
            tree,
        }
    }

    /// Returns a `UiBuilder` to build the UI. New widgets
    /// are added to the UI, widgets from the previous
    /// `build()` call are persited, and missing widgets are removed.
    pub fn build(&mut self) -> UiBuilder {
        self.tree.children.clear();
        self.tree.roots.clear();
        for (_, slot) in self.tree.nodes.drain() {
            self.stretch.remove(slot.stretch_node);
        }
        UiBuilder {
            ui: self,
            parent_stack: Vec::new(),
        }
    }

    /// Renders to the canvas. Does not clear.
    pub fn render(&mut self, canvas: &mut Canvas) {
        self.compute_layout(canvas.width(), canvas.height());
        let Self { stretch, .. } = self;
        self.tree
            .fold_traverse(Vec2::zero(), |parent_pos, _id, slot| {
                let layout = stretch.layout(slot.stretch_node).unwrap();
                let bounds = Rect {
                    pos: vec2(layout.location.x, layout.location.y) + parent_pos,
                    size: vec2(layout.size.width, layout.size.height),
                };
                slot.node.borrow_mut().draw(bounds, canvas);
                parent_pos + bounds.pos
            });
    }

    fn compute_layout(&mut self, width: f32, height: f32) {
        self.stretch
            .compute_layout(
                self.root_stretch_node,
                Size {
                    width: Number::Defined(width),
                    height: Number::Defined(height),
                },
            )
            .unwrap();
    }

    fn insert_node(
        &mut self,
        parent: Option<NodeId>,
        node: Rc<RefCell<dyn WidgetState>>,
    ) -> NodeId {
        let stretch_node = self.create_stretch_node(&node);
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

        id
    }

    fn create_stretch_node(&mut self, node_rc: &Rc<RefCell<dyn WidgetState>>) -> Node {
        let node = node_rc.borrow();
        if node.is_leaf() {
            let node_rc = Rc::clone(node_rc);
            let measure = Box::new(move |max_size: stretch::geometry::Size<Number>| {
                let max_width = match max_size.width {
                    Number::Defined(x) => Some(x),
                    Number::Undefined => None,
                };
                let max_height = match max_size.height {
                    Number::Defined(x) => Some(x),
                    Number::Undefined => None,
                };
                let size = node_rc.borrow_mut().compute_size(max_width, max_height);
                Ok(Size {
                    width: size.x,
                    height: size.y,
                })
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
    parent_stack: Vec<NodeId>,
}

impl<'a> UiBuilder<'a> {
    /// Pushes a new child node to the current parent.
    pub fn push<D>(&mut self, data: D) -> &mut Self
    where
        D: WidgetData,
        D::State: WidgetState + 'static,
    {
        let node = data.into_state();
        self.ui.insert_node(
            self.parent_stack.last().copied(),
            Rc::new(RefCell::new(node)),
        );
        self
    }

    /// Pushes a new child node to the current parent, and sets
    /// the current parent as the new node.
    pub fn begin<D>(&mut self, data: D) -> &mut Self
    where
        D: WidgetData,
        D::State: WidgetState + 'static,
    {
        let node = data.into_state();
        let id = self.ui.insert_node(
            self.parent_stack.last().copied(),
            Rc::new(RefCell::new(node)),
        );
        self.parent_stack.push(id);
        self
    }

    /// Ends the current parent and pops it from the parent stack,
    /// allowing new siblings to be added.
    pub fn end(&mut self) -> &mut Self {
        self.parent_stack.pop();
        self
    }
}

struct NodeSlot {
    node: Rc<RefCell<dyn WidgetState>>,
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
    pub fn fold_traverse<S: Copy>(
        &mut self,
        start_state: S,
        mut callback: impl FnMut(S, NodeId, &mut NodeSlot) -> S,
    ) {
        let mut stack: Vec<_> = self.roots.iter().map(|&root| (root, start_state)).collect();
        while let Some((id, state)) = stack.pop() {
            let slot = self.nodes.get_mut(&id).unwrap();
            let new_state = callback(state, id, slot);
            stack.extend(
                self.children
                    .get(&id)
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                    .iter()
                    .map(|&child| (child, new_state)),
            );
        }
    }
}
