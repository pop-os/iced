use crate::handlers::SctkState;
use sctk::delegate_subcompositor;
use std::fmt::Debug;

delegate_subcompositor!(@<T: 'static + Debug> SctkState<T>);
