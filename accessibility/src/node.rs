use crate::A11yId;

#[derive(Debug, Clone, PartialEq)]
pub struct A11yNode {
    node: accesskit::Node,
    id: A11yId,
}

impl A11yNode {
    pub fn new<T: Into<A11yId>>(node: accesskit::Node, id: T) -> Self {
        Self {
            node,
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
        let mut children =
            children.into_iter().map(|id| id.into()).collect::<Vec<_>>();
        children.extend_from_slice(self.node.children());
        self.node.set_children(children);
    }
}

impl From<A11yNode> for (accesskit::NodeId, accesskit::Node) {
    fn from(node: A11yNode) -> Self {
        (node.id.into(), node.node)
    }
}
