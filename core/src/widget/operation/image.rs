//! Operate on widgets that display an image.
use crate::widget::Id;

use crate::image::Handle;

use super::Operation;

/// The internal state of a widget that displays an image.
pub trait Image {
    /// Sets the handle of the image.
    fn set_handle(&mut self, handle: Handle);
}

/// Produces an [`Operation`] that sets the handle of the widget with the given [`Id`].
pub fn set_handle<T>(target: Id, handle: Handle) -> impl Operation<T> {
    struct SetHandle {
        target: Id,
        handle: Handle,
    }

    impl<T> Operation<T> for SetHandle {
        fn image(&mut self, state: &mut dyn Image, id: Option<&Id>) {
            match id {
                Some(id) if id == &self.target => {
                    state.set_handle(self.handle.clone());
                }
                _ => println!("Invalid id for image widget: {:?}", id),
            }
        }

        fn container(
            &mut self,
            _id: Option<&Id>,
            _bounds: crate::Rectangle,
            operate_on_children: &mut dyn FnMut(&mut dyn Operation<T>),
        ) {
            operate_on_children(self);
        }
    }

    SetHandle { target, handle }
}
