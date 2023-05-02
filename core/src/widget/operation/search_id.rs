//! Search for widgets with the target Id.

use super::Operation;
use crate::{id::Id, widget::operation::Outcome, Rectangle};

/// Produces an [`Operation`] that searches for the Id
pub fn search_id(target: Id) -> impl Operation<Id> {
    struct Find {
        found: bool,
        target: Id,
    }

    impl Operation<Id> for Find {
        fn custom(&mut self, _state: &mut dyn std::any::Any, id: Option<&Id>) {
            if Some(&self.target) == id {
                self.found = true;
            }
        }

        fn container(
            &mut self,
            _id: Option<&Id>,
            _bounds: Rectangle,
            operate_on_children: &mut dyn FnMut(&mut dyn Operation<Id>),
        ) {
            operate_on_children(self);
        }

        fn finish(&self) -> Outcome<Id> {
            if self.found {
                Outcome::Some(self.target.clone())
            } else {
                Outcome::None
            }
        }
    }

    Find {
        found: false,
        target,
    }
}
