use crate::futures::futures::channel::mpsc;
use crate::program::Control;

use iced_accessibility::accesskit::{
    ActivationHandler, NodeBuilder, NodeId, Role, Tree, TreeUpdate,
};
use iced_accessibility::accesskit_winit::Adapter;
use iced_runtime::core;

pub struct WinitActivationHandler {
    pub proxy: mpsc::UnboundedSender<Control>,
    pub title: String,
}

impl ActivationHandler for WinitActivationHandler {
    fn request_initial_tree(
        &mut self,
    ) -> Option<iced_accessibility::accesskit::TreeUpdate> {
        let node_id = core::id::window_node_id();

        let _ = self
            .proxy
            .unbounded_send(Control::AccessibilityEnabled(true));
        let mut node = NodeBuilder::new(Role::Window);
        node.set_name(self.title.clone());
        let node = node.build();
        let root = NodeId(node_id);
        Some(TreeUpdate {
            nodes: vec![(root, node)],
            tree: Some(Tree::new(root)),
            focus: root,
        })
    }
}

pub struct WinitActionHandler {
    pub id: core::window::Id,
    pub proxy: mpsc::UnboundedSender<Control>,
}

impl iced_accessibility::accesskit::ActionHandler for WinitActionHandler {
    fn do_action(
        &mut self,
        request: iced_accessibility::accesskit::ActionRequest,
    ) {
        let _ = self
            .proxy
            .unbounded_send(Control::Accessibility(self.id, request));
    }
}

pub struct WinitDeactivationHandler {
    pub proxy: mpsc::UnboundedSender<Control>,
}

impl iced_accessibility::accesskit::DeactivationHandler
    for WinitDeactivationHandler
{
    fn deactivate_accessibility(&mut self) {
        let _ = self
            .proxy
            .unbounded_send(Control::AccessibilityEnabled(false));
    }
}
