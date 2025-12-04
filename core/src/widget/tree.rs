//! Store internal widget state in a state tree to ensure continuity.
use crate::id::{Id, Internal};
use crate::Widget;
use std::any::{self, Any};
use std::borrow::{Borrow, BorrowMut, Cow};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::{fmt, mem};

thread_local! {
    /// A map of named widget states.
pub static NAMED: std::cell::RefCell<HashMap<Cow<'static, str>, (State, Vec<(usize, Tree)>)>> = std::cell::RefCell::new(HashMap::new());
}

impl Default for Tree {
    fn default() -> Self {
        Self::empty()
    }
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

    pub fn take_all_named(
        &mut self,
    ) -> HashMap<Cow<'static, str>, (State, Vec<(usize, Tree)>)> {
        #[derive(Debug, Clone)]
        enum NodeParent {
            Named(Cow<'static, str>),
            Unnamed(u32),
        }

        enum Node<'a> {
            Tree(&'a mut Tree),
            Unnamed {
                parent: NodeParent,
                unnamed_id: u32,
                child_index: Option<usize>,
            },
        }

        // Helper: create an unnamed Node
        fn make_unnamed_node(
            parent: NodeParent,
            child_index: Option<usize>,
            tree: &mut Tree,
            unnamed: &mut HashMap<u32, Tree>,
            ctr: &mut u32,
        ) -> Node<'static> {
            _ = ctr.checked_add(1).unwrap();
            let tree = mem::replace(
                tree,
                Tree {
                    id: tree.id.clone(),
                    tag: tree.tag,
                    ..Tree::empty()
                },
            );
            _ = unnamed.insert(*ctr, tree);
            Node::Unnamed {
                parent,
                unnamed_id: *ctr,
                child_index,
            }
        }

        let mut named = HashMap::new();
        struct Visit {
            parent: Cow<'static, str>,
            index: usize,
            visited: bool,
        }
        let mut unnamed_id_ctr: u32 = 0;
        let mut unnamed = HashMap::new();
        // tree traversal to find all named widgets
        // and keep their state and children
        let mut stack: Vec<(Node, Option<Visit>)> =
            vec![(Node::Tree(self), None)];
        let mut canary: i32 = 0;
        while let Some((node, visit)) = stack.pop() {
            match node {
                Node::Tree(tree) => {
                    if let Some(Id(Internal::Custom(_, n))) = tree.id.as_ref() {
                        let state = mem::replace(&mut tree.state, State::None);
                        let children_count = tree.children.len();
                        let children =
                            tree.children.iter_mut().enumerate().rev().map(
                                |(i, c)| {
                                    if matches!(
                                        c.id,
                                        Some(Id(Internal::Custom(_, _)))
                                    ) {
                                        (Node::Tree(c), None)
                                    } else {
                                        (
                                            Node::Tree(c),
                                            Some(Visit {
                                                index: i,
                                                parent: n.clone(),
                                                visited: false,
                                            }),
                                        )
                                    }
                                },
                            );
                        _ = named.insert(
                            n.clone(),
                            (state, Vec::with_capacity(children_count)),
                        );
                        stack.extend(children);
                    } else if let Some(visit) = visit {
                        if visit.visited {
                            canary -= 1;
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
                            canary += 1;

                            let node = make_unnamed_node(
                                NodeParent::Named(visit.parent.clone()),
                                None,
                                tree,
                                &mut unnamed,
                                &mut unnamed_id_ctr,
                            );
                            stack.push((
                                node,
                                Some(Visit {
                                    visited: true,
                                    parent: visit.parent.clone(),
                                    ..visit
                                }),
                            ));
                            let unnamed_id = unnamed_id_ctr;
                            stack.extend((0..tree.children.len()).map(|i| {
                                (
                                    Node::Unnamed {
                                        parent: NodeParent::Unnamed(unnamed_id),
                                        unnamed_id: unnamed_id_ctr,
                                        child_index: Some(i),
                                    },
                                    None,
                                )
                            }));
                        }
                    } else {
                        stack.extend(
                            tree.children
                                .iter_mut()
                                .map(|s| (Node::Tree(s), None)),
                        );
                    }
                }
                Node::Unnamed {
                    parent,
                    unnamed_id,
                    child_index,
                } => {
                    if let Some(visit) = visit {
                        if visit.visited {
                            match parent {
                                NodeParent::Named(name) => {
                                    canary -= 1;
                                    let tree =
                                        unnamed.get_mut(&unnamed_id).unwrap();
                                    let id = tree.id.clone();
                                    let tag = tree.tag;
                                    named.get_mut(&name).unwrap().1.push((
                                        visit.index,
                                        mem::replace(
                                            tree,
                                            Tree {
                                                id,
                                                tag,
                                                ..Tree::empty()
                                            },
                                        ),
                                    ));
                                }
                                NodeParent::Unnamed(parent_unnamed_id) => {
                                    canary -= 1;

                                    let mut tree =
                                        unnamed.remove(&unnamed_id).unwrap();

                                    let parent_tree = unnamed
                                        .get_mut(&parent_unnamed_id)
                                        .unwrap();
                                    let id = tree.id.clone();
                                    let tag = tree.tag;

                                    parent_tree.children[visit.index] =
                                        mem::replace(
                                            &mut tree,
                                            Tree {
                                                id,
                                                tag,
                                                ..Tree::empty()
                                            },
                                        );
                                    _ = unnamed.insert(unnamed_id, tree);
                                }
                            }
                        } else if let Some(child_index) = child_index {
                            // this is the first visit, of an unnamed child.
                            // the tree is actually the parent tree. so we need to get it using the child index.
                            // need to push children
                            let mut tree = unnamed.remove(&unnamed_id).unwrap();

                            let id = tree.children[child_index].id.clone();
                            let tag = tree.children[child_index].tag;
                            let mut my_tree = mem::replace(
                                &mut tree.children[child_index],
                                Tree {
                                    id,
                                    tag,
                                    ..Tree::empty()
                                },
                            );
                            // ???
                            _ = unnamed.insert(unnamed_id, tree);
                            // this child might be named, so we need to check that.
                            if let Some(Id(Internal::Custom(_, ref name))) =
                                my_tree.id
                            {
                                canary += 1;

                                let state = mem::replace(
                                    &mut my_tree.state,
                                    State::None,
                                );
                                let children_count = my_tree.children.len();

                                // handle the children now
                                let children = my_tree
                                    .children
                                    .iter_mut()
                                    .enumerate()
                                    .rev()
                                    .map(|(i, c)| {
                                        canary += 1;

                                        let node = make_unnamed_node(
                                            NodeParent::Named(name.clone()),
                                            None,
                                            c,
                                            &mut unnamed,
                                            &mut unnamed_id_ctr,
                                        );

                                        if matches!(
                                            c.id,
                                            Some(Id(Internal::Custom(_, _)))
                                        ) {
                                            (node, None)
                                        } else {
                                            (
                                                node,
                                                Some(Visit {
                                                    index: i,
                                                    parent: name.clone(),
                                                    visited: false,
                                                }),
                                            )
                                        }
                                    });
                                stack.extend(children);

                                _ = named.insert(
                                    name.clone(),
                                    (state, Vec::with_capacity(children_count)),
                                );
                            } else {
                                canary += 1;

                                // add a new counter and insert into unnamed
                                // keep track of parent

                                let node = make_unnamed_node(
                                    NodeParent::Unnamed(unnamed_id),
                                    None,
                                    &mut my_tree,
                                    &mut unnamed,
                                    &mut unnamed_id_ctr,
                                );
                                stack.push((
                                    node,
                                    Some(Visit {
                                        visited: true,
                                        parent: visit.parent.clone(),
                                        ..visit
                                    }),
                                ));
                                let unnamed_id = unnamed_id_ctr;
                                // push children
                                stack.extend((0..my_tree.children.len()).map(
                                    |i| {
                                        canary += 1;
                                        let node = make_unnamed_node(
                                            NodeParent::Unnamed(unnamed_id),
                                            Some(i),
                                            &mut my_tree,
                                            &mut unnamed,
                                            &mut unnamed_id_ctr,
                                        );
                                        (node, None)
                                    },
                                ));
                                _ = unnamed.insert(unnamed_id_ctr, my_tree);
                            }
                            panic!(
                                "Invalid state in tree traversal: visit={:?}, child_index={:?}, parent={:?}, unnamed_id={:?}",
                                visit.index, child_index, parent, unnamed_id
                            );
                        }
                    } else {
                        canary += 1;
                        let parent_tree = unnamed.get_mut(&unnamed_id).unwrap();

                        // must be from a named parent that is the child of an unnamed widget
                        // so we just push all children
                        let mut to_insert =
                            Vec::with_capacity(parent_tree.children.len());
                        stack.extend((0..parent_tree.children.len()).map(
                            |i| {
                                canary += 1;

                                // increment unnamed_id counter
                                unnamed_id_ctr =
                                    unnamed_id_ctr.checked_add(1).unwrap();
                                // take the child tree and insert to unnamed
                                let my_tree = mem::replace(
                                    &mut parent_tree.children[i],
                                    Tree::empty(),
                                );
                                let child_unnamed_id = unnamed_id_ctr;
                                _ = to_insert.push((child_unnamed_id, my_tree));
                                (
                                    Node::Unnamed {
                                        parent: NodeParent::Unnamed(unnamed_id),
                                        unnamed_id: child_unnamed_id,
                                        child_index: Some(i),
                                    },
                                    None,
                                )
                            },
                        ));

                        // insert parent with a visit: true
                        stack.push((
                            Node::Unnamed {
                                parent: parent.clone(),
                                unnamed_id,
                                child_index: None,
                            },
                            Some(Visit {
                                visited: true,
                                parent: match parent {
                                    NodeParent::Unnamed(id) => {
                                        if let Some(t) = unnamed.get(&id) {
                                            if let Some(Id(Internal::Custom(
                                                _,
                                                ref name,
                                            ))) = t.id
                                            {
                                                name.clone()
                                            } else {
                                                Cow::Borrowed("")
                                            }
                                        } else {
                                            Cow::Borrowed("")
                                        }
                                    }
                                    NodeParent::Named(name) => name.clone(),
                                },
                                index: 0,
                            }),
                        ));
                        for (id, tree) in to_insert {
                            _ = unnamed.insert(id, tree);
                        }
                    }
                }
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
                    if !tag_match
                        || self.children.len() != widget_children.len()
                    {
                        self.children = borrowed.children();
                    } else {
                        for (old_i, mut old) in children {
                            let Some(my_state) = self.children.get_mut(old_i)
                            else {
                                continue;
                            };
                            if my_state.tag != old.tag || {
                                !match (&old.id, &my_state.id) {
                                    (
                                        Some(Id(Internal::Custom(
                                            _,
                                            ref old_name,
                                        ))),
                                        Some(Id(Internal::Custom(
                                            _,
                                            ref my_name,
                                        ))),
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
                if self.children.len() != borrowed.children().len() {
                    self.children = borrowed.children();
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
        if self.children.len() > new_children.len() {
            self.children.truncate(new_children.len());
        }

        let children_len = self.children.len();
        let (mut id_map, mut id_list): (
            HashMap<String, &mut Tree>,
            VecDeque<(usize, &mut Tree)>,
        ) = self.children.iter_mut().enumerate().fold(
            (HashMap::new(), VecDeque::with_capacity(children_len)),
            |(mut id_map, mut id_list), (i, c)| {
                if let Some(id) = c.id.as_ref() {
                    if let Internal::Custom(_, ref name) = id.0 {
                        let _ = id_map.insert(name.to_string(), c);
                    } else {
                        id_list.push_back((i, c));
                    }
                } else {
                    id_list.push_back((i, c));
                }
                (id_map, id_list)
            },
        );

        let mut new_trees: Vec<(Tree, usize)> =
            Vec::with_capacity(new_children.len());
        for (i, (new, new_id)) in
            new_children.iter_mut().zip(new_ids.iter()).enumerate()
        {
            let child_state = if let Some(c) = new_id.as_ref().and_then(|id| {
                if let Internal::Custom(_, ref name) = id.0 {
                    id_map.remove(name.as_ref())
                } else {
                    None
                }
            }) {
                c
            } else if let Some(i) = {
                let mut found = None;
                for c_i in 0..id_list.len() {
                    if id_list[c_i].0 == i {
                        found = Some(c_i);
                        break;
                    }
                    if i < c_i {
                        break;
                    }
                }
                found
            } {
                let c = id_list.remove(i).unwrap().1;
                c
            } else {
                let mut my_new_state = new_state(new);
                diff(&mut my_new_state, new);
                new_trees.push((my_new_state, i));
                continue;
            };

            diff(child_state, new);
        }

        for (new_tree, i) in new_trees {
            if self.children.len() > i {
                self.children[i] = new_tree;
            } else {
                self.children.push(new_tree);
            }
        }
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
