//! Store internal widget state in a state tree to ensure continuity.
use crate::Widget;
use crate::id::{Id, Internal};
use std::any::{self, Any};
use std::borrow::{Borrow, BorrowMut, Cow};
use std::collections::HashMap;
use std::hash::Hash;
use std::{fmt, mem};

thread_local! {
    /// A map of named widget states.
pub static NAMED: std::cell::RefCell<HashMap<Cow<'static, str>, (State, Vec<(usize, Tree)>)>> = std::cell::RefCell::new(HashMap::new());
}

/// A persistent state widget tree.
///
/// A [`Tree`] is normally associated with a specific widget in the widget tree.
#[derive(Debug)]
pub struct Tree {
    /// The tag of the [`Tree`].
    pub tag: Tag,

    /// the Id of the [`Tree`]
    pub id: Option<Id>,

    /// The [`State`] of the [`Tree`].
    pub state: State,

    /// The children of the root widget of the [`Tree`].
    pub children: Vec<Tree>,
}

impl Tree {
    /// Creates an empty, stateless [`Tree`] with no children.
    pub fn empty() -> Self {
        Self {
            id: None,
            tag: Tag::stateless(),
            state: State::None,
            children: Vec::new(),
        }
    }

    /// Creates a new [`Tree`] for the provided [`Widget`].
    pub fn new<'a, Message, Theme, Renderer>(
        widget: impl Borrow<dyn Widget<Message, Theme, Renderer> + 'a>,
    ) -> Self
    where
        Renderer: crate::Renderer,
    {
        let widget = widget.borrow();

        Self {
            id: widget.id(),
            tag: widget.tag(),
            state: widget.state(),
            children: widget.children(),
        }
    }

    /// Takes all named widgets from the tree.
    pub fn take_all_named(
        &mut self,
    ) -> HashMap<Cow<'static, str>, (State, Vec<(usize, Tree)>)> {
        let mut named = HashMap::new();
        struct Visit {
            parent: Cow<'static, str>,
            index: usize,
            visited: bool,
        }
        // tree traversal to find all named widgets
        // and keep their state and children
        let mut stack = vec![(self, None)];
        while let Some((tree, visit)) = stack.pop() {
            if let Some(Id(Internal::Custom(_, n))) = tree.id.clone() {
                let state = mem::replace(&mut tree.state, State::None);
                let children_count = tree.children.len();
                let children =
                    tree.children.iter_mut().enumerate().rev().map(|(i, c)| {
                        if matches!(c.id, Some(Id(Internal::Custom(_, _)))) {
                            (c, None)
                        } else {
                            (
                                c,
                                Some(Visit {
                                    index: i,
                                    parent: n.clone(),
                                    visited: false,
                                }),
                            )
                        }
                    });
                _ = named.insert(
                    n.clone(),
                    (state, Vec::with_capacity(children_count)),
                );
                stack.extend(children);
            } else if let Some(visit) = visit {
                if visit.visited {
                    named.get_mut(&visit.parent).unwrap().1.push((
                        visit.index,
                        mem::replace(
                            tree,
                            Tree {
                                id: tree.id.clone(),
                                tag: tree.tag,
                                ..Tree::empty()
                            },
                        ),
                    ));
                } else {
                    let ptr = tree as *mut Tree;

                    stack.push((
                        // TODO remove this unsafe block
                        #[allow(unsafe_code)]
                        // SAFETY: when the reference is finally accessed, all the children references will have been processed first.
                        unsafe {
                            ptr.as_mut().unwrap()
                        },
                        Some(Visit {
                            visited: true,
                            ..visit
                        }),
                    ));
                    stack.extend(tree.children.iter_mut().map(|c| (c, None)));
                }
            } else {
                stack.extend(tree.children.iter_mut().map(|s| (s, None)));
            }
        }

        named
    }

    /// Finds a widget state in the tree by its id.
    pub fn find<'a>(&'a self, id: &Id) -> Option<&'a Tree> {
        if self.id == Some(id.clone()) {
            return Some(self);
        }

        for child in self.children.iter() {
            if let Some(tree) = child.find(id) {
                return Some(tree);
            }
        }

        None
    }
    /// Reconciliates the current tree with the provided [`Widget`].
    ///
    /// If the tag of the [`Widget`] matches the tag of the [`Tree`], then the
    /// [`Widget`] proceeds with the reconciliation (i.e. [`Widget::diff`] is called).
    ///
    /// Otherwise, the whole [`Tree`] is recreated.
    ///
    /// [`Widget::diff`]: crate::Widget::diff
    pub fn diff<'a, Message, Theme, Renderer>(
        &mut self,
        mut new: impl BorrowMut<dyn Widget<Message, Theme, Renderer> + 'a>,
    ) where
        Renderer: crate::Renderer,
    {
        let borrowed: &mut dyn Widget<Message, Theme, Renderer> =
            new.borrow_mut();

        let mut tag_match = self.tag == borrowed.tag();

        if tag_match {
            if let Some(Id(Internal::Custom(_, n))) = borrowed.id() {
                if let Some((mut state, children)) = NAMED
                    .with(|named| named.borrow_mut().remove(&n))
                    .or_else(|| {
                        //check self.id
                        if let Some(Id(Internal::Custom(_, ref name))) = self.id
                        {
                            if name == &n {
                                Some((
                                    mem::replace(&mut self.state, State::None),
                                    self.children
                                        .iter_mut()
                                        .map(|s| {
                                            // take the data
                                            mem::replace(
                                                s,
                                                Tree {
                                                    id: s.id.clone(),
                                                    tag: s.tag,
                                                    ..Tree::empty()
                                                },
                                            )
                                        })
                                        .enumerate()
                                        .collect(),
                                ))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                {
                    std::mem::swap(&mut self.state, &mut state);
                    let widget_children = borrowed.children();
                    if !tag_match {
                        self.children = widget_children;
                    } else {
                        for (old_i, mut old) in children {
                            let Some(my_state) = self.children.get_mut(old_i)
                            else {
                                continue;
                            };
                            if my_state.tag != old.tag || {
                                !match (&old.id, &my_state.id) {
                                    (
                                        Some(Id(Internal::Custom(_, old_name))),
                                        Some(Id(Internal::Custom(_, my_name))),
                                    ) => old_name == my_name,
                                    (
                                        Some(Id(Internal::Set(a))),
                                        Some(Id(Internal::Set(b))),
                                    ) => a.len() == b.len(),
                                    (
                                        Some(Id(Internal::Unique(_))),
                                        Some(Id(Internal::Unique(_))),
                                    ) => true,
                                    (None, None) => true,
                                    _ => false,
                                }
                            } {
                                continue;
                            }

                            mem::swap(my_state, &mut old);
                        }
                    }
                } else {
                    tag_match = false;
                }
            } else {
                if let Some(id) = self.id.clone() {
                    borrowed.set_id(id);
                }
            }
        }
        if tag_match {
            borrowed.diff(self);
        } else {
            *self = Self::new(borrowed);
            let borrowed = new.borrow_mut();
            borrowed.diff(self);
        }
    }

    /// Reconciles the children of the tree with the provided list of widgets.
    pub fn diff_children<'a, Message, Theme, Renderer>(
        &mut self,
        new_children: &mut [impl BorrowMut<
            dyn Widget<Message, Theme, Renderer> + 'a,
        >],
    ) where
        Renderer: crate::Renderer,
    {
        self.diff_children_custom(
            new_children,
            new_children.iter().map(|c| c.borrow().id()).collect(),
            |tree, widget| {
                let borrowed: &mut dyn Widget<_, _, _> = widget.borrow_mut();

                tree.diff(borrowed);
            },
            |widget| {
                let borrowed: &dyn Widget<_, _, _> = widget.borrow();
                Self::new(borrowed)
            },
        );
    }

    /// Reconciles the children of the tree with the provided list of widgets using custom
    /// logic both for diffing and creating new widget state.
    pub fn diff_children_custom<T>(
        &mut self,
        new_children: &mut [T],
        new_ids: Vec<Option<Id>>,
        diff: impl Fn(&mut Tree, &mut T),
        new_state: impl Fn(&T) -> Self,
    ) {
        let old_children = mem::take(&mut self.children);

        let mut named: HashMap<Cow<'static, str>, Tree> = HashMap::new();
        let mut positional: Vec<Option<Tree>> =
            Vec::with_capacity(old_children.len());

        for old_child in old_children {
            if let Some(Id(Internal::Custom(_, ref name))) = old_child.id {
                let _ = named.insert(name.clone(), old_child);
                positional.push(None);
            } else {
                positional.push(Some(old_child));
            }
        }

        self.children = new_children
            .iter_mut()
            .zip(new_ids)
            .enumerate()
            .map(|(index, (new, new_id))| {
                let matched =
                    if let Some(Id(Internal::Custom(_, ref name))) = new_id {
                        named.remove(name.as_ref())
                    } else {
                        positional.get_mut(index).and_then(Option::take)
                    };

                let mut tree = matched.unwrap_or_else(|| new_state(new));
                diff(&mut tree, new);
                tree
            })
            .collect();
    }
}

/// Reconciles the `current_children` with the provided list of widgets using
/// custom logic both for diffing and creating new widget state.
///
/// The algorithm will try to minimize the impact of diffing by querying the
/// `maybe_changed` closure.
pub fn diff_children_custom_with_search<T>(
    current_children: &mut Vec<Tree>,
    new_children: &mut [T],
    diff: impl Fn(&mut Tree, &mut T),
    maybe_changed: impl Fn(usize) -> bool,
    new_state: impl Fn(&T) -> Tree,
) {
    if new_children.is_empty() {
        current_children.clear();
        return;
    }

    if current_children.is_empty() {
        current_children.extend(new_children.iter().map(new_state));
        return;
    }

    let first_maybe_changed = maybe_changed(0);
    let last_maybe_changed = maybe_changed(current_children.len() - 1);

    if current_children.len() > new_children.len() {
        if !first_maybe_changed && last_maybe_changed {
            current_children.truncate(new_children.len());
        } else {
            let difference_index = if first_maybe_changed {
                0
            } else {
                (1..current_children.len())
                    .find(|&i| maybe_changed(i))
                    .unwrap_or(0)
            };

            let _ = current_children.splice(
                difference_index
                    ..difference_index
                        + (current_children.len() - new_children.len()),
                std::iter::empty(),
            );
        }
    }

    if current_children.len() < new_children.len() {
        let first_maybe_changed = maybe_changed(0);
        let last_maybe_changed = maybe_changed(current_children.len() - 1);

        if !first_maybe_changed && last_maybe_changed {
            current_children.extend(
                new_children[current_children.len()..].iter().map(new_state),
            );
        } else {
            let difference_index = if first_maybe_changed {
                0
            } else {
                (1..current_children.len())
                    .find(|&i| maybe_changed(i))
                    .unwrap_or(0)
            };

            let _ = current_children.splice(
                difference_index..difference_index,
                new_children[difference_index
                    ..difference_index
                        + (new_children.len() - current_children.len())]
                    .iter()
                    .map(new_state),
            );
        }
    }

    // TODO: Merge loop with extend logic (?)
    for (child_state, new) in
        current_children.iter_mut().zip(new_children.iter_mut())
    {
        diff(child_state, new);
    }
}

/// The identifier of some widget state.
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Tag(any::TypeId);

impl Tag {
    /// Creates a [`Tag`] for a state of type `T`.
    pub fn of<T>() -> Self
    where
        T: 'static,
    {
        Self(any::TypeId::of::<T>())
    }

    /// Creates a [`Tag`] for a stateless widget.
    pub fn stateless() -> Self {
        Self::of::<()>()
    }
}

/// The internal [`State`] of a widget.
pub enum State {
    /// No meaningful internal state.
    None,

    /// Some meaningful internal state.
    Some(Box<dyn Any>),
}

impl State {
    /// Creates a new [`State`].
    pub fn new<T>(state: T) -> Self
    where
        T: 'static,
    {
        State::Some(Box::new(state))
    }

    /// Downcasts the [`State`] to `T` and returns a reference to it.
    ///
    /// # Panics
    /// This method will panic if the downcast fails or the [`State`] is [`State::None`].
    pub fn downcast_ref<T>(&self) -> &T
    where
        T: 'static,
    {
        match self {
            State::None => panic!("Downcast on stateless state"),
            State::Some(state) => {
                state.downcast_ref().expect("Downcast widget state")
            }
        }
    }

    /// Downcasts the [`State`] to `T` and returns a mutable reference to it.
    ///
    /// # Panics
    /// This method will panic if the downcast fails or the [`State`] is [`State::None`].
    pub fn downcast_mut<T>(&mut self) -> &mut T
    where
        T: 'static,
    {
        match self {
            State::None => panic!("Downcast on stateless state"),
            State::Some(state) => {
                state.downcast_mut().expect("Downcast widget state")
            }
        }
    }
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "State::None"),
            Self::Some(_) => write!(f, "State::Some"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRESH: u32 = 999;

    fn named_tree(name: &'static str, value: u32) -> Tree {
        Tree {
            tag: Tag::of::<u32>(),
            id: Some(Id::new(name)),
            state: State::new(value),
            children: Vec::new(),
        }
    }

    fn unnamed_tree(value: u32) -> Tree {
        Tree {
            tag: Tag::of::<u32>(),
            id: None,
            state: State::new(value),
            children: Vec::new(),
        }
    }

    fn parent_of(children: Vec<Tree>) -> Tree {
        Tree {
            children,
            ..Tree::empty()
        }
    }

    fn state_of(tree: &Tree) -> u32 {
        *tree.state.downcast_ref::<u32>()
    }

    /// Reconciles `parent` against children identified by `new_ids`,
    /// giving any unmatched child a `FRESH` state.
    fn reconcile(parent: &mut Tree, new_ids: Vec<Option<Id>>) {
        let mut new_children = new_ids.clone();
        parent.diff_children_custom(
            &mut new_children,
            new_ids,
            |_tree, _new| {},
            |new| Tree {
                tag: Tag::of::<u32>(),
                id: new.clone(),
                state: State::new(FRESH),
                children: Vec::new(),
            },
        );
    }

    #[test]
    fn insertion_moves_named_state_to_its_new_index() {
        let mut parent = parent_of(vec![unnamed_tree(10), named_tree("x", 20)]);

        reconcile(&mut parent, vec![None, None, Some(Id::new("x"))]);

        assert_eq!(parent.children.len(), 3);
        assert_eq!(state_of(&parent.children[0]), 10);
        assert_eq!(state_of(&parent.children[1]), FRESH);
        assert_eq!(state_of(&parent.children[2]), 20);
        assert_eq!(parent.children[2].id, Some(Id::new("x")));
    }

    #[test]
    fn removal_keeps_named_state_when_it_shifts_left() {
        let mut parent = parent_of(vec![
            unnamed_tree(10),
            unnamed_tree(11),
            named_tree("x", 20),
        ]);

        reconcile(&mut parent, vec![None, Some(Id::new("x"))]);

        assert_eq!(parent.children.len(), 2);
        assert_eq!(state_of(&parent.children[0]), 10);
        assert_eq!(state_of(&parent.children[1]), 20);
    }

    #[test]
    fn reorder_swaps_named_states() {
        let mut parent =
            parent_of(vec![named_tree("a", 1), named_tree("b", 2)]);

        reconcile(&mut parent, vec![Some(Id::new("b")), Some(Id::new("a"))]);

        assert_eq!(state_of(&parent.children[0]), 2);
        assert_eq!(state_of(&parent.children[1]), 1);
    }

    #[test]
    fn positional_children_keep_their_state() {
        let mut parent = parent_of(vec![unnamed_tree(10), unnamed_tree(11)]);

        reconcile(&mut parent, vec![None, None]);

        assert_eq!(state_of(&parent.children[0]), 10);
        assert_eq!(state_of(&parent.children[1]), 11);
    }

    #[test]
    fn positional_child_wont_inherit_state_of_named() {
        let mut parent = parent_of(vec![named_tree("a", 1)]);

        reconcile(&mut parent, vec![None]);

        assert_eq!(parent.children.len(), 1);
        assert_eq!(state_of(&parent.children[0]), FRESH);
    }

    #[test]
    fn named_child_wont_inherit_state_of_positional() {
        let mut parent = parent_of(vec![unnamed_tree(10)]);

        reconcile(&mut parent, vec![Some(Id::new("x"))]);

        assert_eq!(parent.children.len(), 1);
        assert_eq!(state_of(&parent.children[0]), FRESH);
    }

    #[test]
    fn excess_children_are_dropped() {
        let mut parent = parent_of(vec![
            unnamed_tree(10),
            named_tree("a", 1),
            unnamed_tree(11),
        ]);

        reconcile(&mut parent, vec![None]);

        assert_eq!(parent.children.len(), 1);
        assert_eq!(state_of(&parent.children[0]), 10);
    }
}
