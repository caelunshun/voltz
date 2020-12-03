use ahash::{AHashMap, AHashSet};
use hecs::{Component, DynamicBundle, Entity, World};
use stretch::{node::Node, style::Style, Stretch};

use crate::Canvas;

/// Globally unique ID of a UI node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(Entity);

/// Builder for a node.
///
/// Created via [`Ui::insert`].
pub struct NodeBuilder<'a> {
    ui: &'a mut Ui,
    node: NodeId,
}

impl<'a> NodeBuilder<'a> {
    /// Adds a child to the node.
    pub fn add_child(
        &mut self,
        child: impl DynamicBundle,
        build: impl FnOnce(NodeBuilder),
    ) -> NodeId {
        let node = self.ui.spawn(child);
        build(NodeBuilder { ui: self.ui, node });
        node
    }
}

/// Component storing a node's children.
struct Children(Vec<NodeId>);

/// A UI. Stores all nodes.
pub struct Ui {
    stretch: Stretch,

    nodes: World,

    added_nodes: Vec<NodeId>,
    updated_nodes: AHashSet<NodeId>,
    queued_for_remove: Vec<NodeId>,

    canvas: Canvas,
}

impl Ui {
    /// Creates a new `Ui` with the given pixel dimensions
    /// and scale factor.
    ///
    /// # Panics
    /// Panics if `width == 0 || height == 0`.
    pub fn new(pixel_width: u32, pixel_height: u32, scale_factor: f32) -> Self {
        let stretch = Stretch::new();
        let nodes = World::new();
        let added_nodes = Vec::new();
        let updated_nodes = AHashSet::new();
        let queued_for_remove = Vec::new();
        let canvas = Canvas::new(pixel_width, pixel_height, scale_factor);

        Self {
            stretch,
            nodes,
            added_nodes,
            updated_nodes,
            queued_for_remove,
            canvas,
        }
    }

    /// Creates a new node.
    pub fn insert(&mut self, node: impl DynamicBundle, build: impl FnOnce(NodeBuilder)) -> NodeId {
        let node = self.spawn(node);
        build(NodeBuilder { ui: self, node });
        node
    }

    /// Removes a node.
    pub fn remove(&mut self, node: NodeId) {
        self.queued_for_remove.push(node);
    }

    /// Gets a component of a node.
    pub fn get<T: Component>(&self, node: NodeId) -> Result<hecs::Ref<T>, hecs::ComponentError> {
        self.nodes.get(node.0)
    }

    /// Gets a component of a node.
    pub fn get_mut<T: Component>(
        &self,
        node: NodeId,
    ) -> Result<hecs::RefMut<T>, hecs::ComponentError> {
        self.updated_nodes.insert(node);
        self.nodes.get_mut(node.0)
    }

    /// Updates the UI. Computes layout, handles events,
    /// and redraws if necessary.
    pub fn update(&mut self) {
        self.update_nodes();
    }

    fn spawn(&mut self, node: impl DynamicBundle) -> NodeId {
        let node = NodeId(self.nodes.spawn(node));
        self.added_nodes.push(node);
        node
    }
}

struct LayoutSurface {
    stretch: Stretch,
    root: Node,
    entity_to_stretch: AHashMap<NodeId, Node>,
}

/// Taken from `bevy-ui` with some changes.
impl LayoutSurface {
    pub fn upsert_node(&mut self, node: NodeId, style: &Style) {
        let mut added = false;
        let stretch = &mut self.stretch;
        let stretch_node = self.entity_to_stretch.entry(node).or_insert_with(|| {
            added = true;
            stretch.new_node(style.clone(), Vec::new()).unwrap()
        });

        if !added {
            self.stretch
                .set_style(*stretch_node, style.clone())
                .unwrap();
        }
    }

    pub fn upsert_leaf(&mut self, entity: Entity, style: &Style, calculated_size: CalculatedSize) {
        let stretch = &mut self.stretch;
        let stretch_style = style.into();
        let measure = Box::new(move |constraints: stretch::geometry::Size<Number>| {
            let mut size = stretch::geometry::Size {
                width: calculated_size.size.width,
                height: calculated_size.size.height,
            };
            match (constraints.width, constraints.height) {
                (Number::Undefined, Number::Undefined) => {}
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

        if let Some(stretch_node) = self.entity_to_stretch.get(&entity) {
            self.stretch
                .set_style(*stretch_node, stretch_style)
                .unwrap();
            self.stretch
                .set_measure(*stretch_node, Some(measure))
                .unwrap();
        } else {
            let stretch_node = stretch.new_leaf(stretch_style, measure).unwrap();
            self.entity_to_stretch.insert(entity, stretch_node);
        }
    }

    pub fn update_children(&mut self, entity: Entity, children: &Children) {
        let mut stretch_children = Vec::with_capacity(children.len());
        for child in children.iter() {
            let stretch_node = self.entity_to_stretch.get(child).unwrap();
            stretch_children.push(*stretch_node);
        }

        let stretch_node = self.entity_to_stretch.get(&entity).unwrap();
        self.stretch
            .set_children(*stretch_node, stretch_children)
            .unwrap();
    }

    pub fn update_window(&mut self, window: &Window) {
        let stretch = &mut self.stretch;
        let node = self.window_nodes.entry(window.id()).or_insert_with(|| {
            stretch
                .new_node(stretch::style::Style::default(), Vec::new())
                .unwrap()
        });

        stretch
            .set_style(
                *node,
                stretch::style::Style {
                    size: stretch::geometry::Size {
                        width: stretch::style::Dimension::Points(window.width() as f32),
                        height: stretch::style::Dimension::Points(window.height() as f32),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
    }

    pub fn set_window_children(
        &mut self,
        window_id: WindowId,
        children: impl Iterator<Item = Entity>,
    ) {
        let stretch_node = self.window_nodes.get(&window_id).unwrap();
        let child_nodes = children
            .map(|e| *self.entity_to_stretch.get(&e).unwrap())
            .collect::<Vec<stretch::node::Node>>();
        self.stretch
            .set_children(*stretch_node, child_nodes)
            .unwrap();
    }

    pub fn compute_window_layouts(&mut self) {
        for window_node in self.window_nodes.values() {
            self.stretch
                .compute_layout(*window_node, stretch::geometry::Size::undefined())
                .unwrap();
        }
    }

    pub fn get_layout(&self, entity: Entity) -> Result<&stretch::result::Layout, stretch::Error> {
        let stretch_node = self.entity_to_stretch.get(&entity).unwrap();
        self.stretch.layout(*stretch_node)
    }
}
