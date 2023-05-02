//! Search for widgets with the target Id.

use super::Operation;
use crate::{
    Rectangle,
    id::Id,
    widget::operation::{Outcome, focusable::Count},
};

/// Produces an [`Operation`] that searches for the Id
pub fn search_id(target: Id) -> impl Operation<Id> {
    struct Find {
        found: bool,
        target: Id,
    }

    impl Operation<Id> for Find {
        fn custom(
            &mut self,
            id: Option<&Id>,
            _bounds: Rectangle,
            _state: &mut dyn std::any::Any,
        ) {
            if Some(&self.target) == id {
                self.found = true;
            }
        }

        fn finish(&self) -> Outcome<Id> {
            if self.found {
                Outcome::Some(self.target.clone())
            } else {
                Outcome::None
            }
        }

        fn traverse(
            &mut self,
            operate: &mut dyn FnMut(&mut dyn Operation<Id>),
        ) {
            operate(self);
        }
    }

    Find {
        found: false,
        target,
    }
}
