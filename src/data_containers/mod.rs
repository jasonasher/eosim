use std::any::Any;
use std::fmt::Debug;

pub mod heterogeneous_container;

pub mod vector_heterogeneous_container;

pub mod person_container;

pub trait Property: Any {
    type Value: Any;
}

pub trait PropertyWithDefault: Any {
    type Value: Any + Clone + Debug;
    fn get_default() -> Self::Value;
}

impl<T: PropertyWithDefault> Property for T {
    type Value = T::Value;
}
