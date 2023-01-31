use std::sync::Arc;

use crate::A11yId;

#[derive(Debug, Clone)]
pub struct A11yNode {
    node: accesskit::Node,
    id: A11yId,
}

impl A11yNode {
    pub fn new<T: Into<A11yId>>(node: accesskit::Node, id: T) -> Self {
        Self {
            node: node,
            id: id.into(),
        }
    }

    pub fn id(&self) -> &A11yId {
        &self.id
    }

    pub fn node_mut(&mut self) -> &mut accesskit::Node {
        &mut self.node
    }

    pub fn node(&self) -> &accesskit::Node {
        &self.node
    }

    pub fn add_children(&mut self, children: Vec<A11yId>) {
        self.node.children.extend(
            children
                .into_iter()
                .map(|id| <A11yId as Into<accesskit::NodeId>>::into(id)),
        );
    }
}

impl Into<(accesskit::NodeId, Arc<accesskit::Node>)> for A11yNode {
    fn into(self) -> (accesskit::NodeId, Arc<accesskit::Node>) {
        (self.id.into(), Arc::new(self.node))
    }
}
