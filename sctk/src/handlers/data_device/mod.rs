use crate::handlers::SctkState;
use sctk::delegate_data_device;
use std::fmt::Debug;

pub mod data_device;
pub mod data_offer;
pub mod data_source;

delegate_data_device!(@<T: 'static + Debug> SctkState<T>);
