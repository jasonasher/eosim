use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::data_containers::PropertyWithDefault;

pub struct VecDataContainer {
    data: HashMap<TypeId, Box<dyn Any>>,
}

impl Default for VecDataContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl VecDataContainer {
    pub fn new() -> VecDataContainer {
        VecDataContainer {
            data: HashMap::new(),
        }
    }

    pub fn set_value<K: PropertyWithDefault>(&mut self, index: usize, value: K::Value) {
        let vec = self
            .data
            .entry(TypeId::of::<K>())
            .or_insert_with(|| Box::new(Vec::<K::Value>::with_capacity(index)));
        let vec: &mut Vec<K::Value> = vec.downcast_mut().unwrap();
        if index >= vec.len() {
            vec.resize(index + 1, K::get_default());
        }
        vec[index] = value;
    }

    pub fn get_value<K: PropertyWithDefault>(&self, index: usize) -> K::Value {
        match self.data.get(&TypeId::of::<K>()) {
            Some(boxed_vec) => {
                let vec = boxed_vec.downcast_ref::<Vec<K::Value>>().unwrap();
                if index >= vec.len() {
                    K::get_default()
                } else {
                    vec[index]
                }
            }
            None => K::get_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct KeyOne {}
    impl PropertyWithDefault for KeyOne {
        type Value = usize;

        fn get_default() -> <Self as PropertyWithDefault>::Value {
            0
        }
    }

    struct KeyTwo {}
    impl PropertyWithDefault for KeyTwo {
        type Value = bool;

        fn get_default() -> <Self as PropertyWithDefault>::Value {
            false
        }
    }

    #[test]
    fn test() {
        use super::*;

        let mut container = VecDataContainer::new();

        assert_eq!(container.get_value::<KeyOne>(0), 0);
        assert_eq!(container.get_value::<KeyTwo>(0), false);

        container.set_value::<KeyOne>(2, 2);
        assert_eq!(container.get_value::<KeyOne>(0), 0);
        assert_eq!(container.get_value::<KeyOne>(1), 0);
        assert_eq!(container.get_value::<KeyOne>(2), 2);
        assert_eq!(container.get_value::<KeyOne>(3), 0);

        container.set_value::<KeyTwo>(2, true);
        assert_eq!(container.get_value::<KeyTwo>(0), false);
        assert_eq!(container.get_value::<KeyTwo>(1), false);
        assert_eq!(container.get_value::<KeyTwo>(2), true);
        assert_eq!(container.get_value::<KeyTwo>(3), false);

        container.set_value::<KeyOne>(1, 3);
        assert_eq!(container.get_value::<KeyOne>(1), 3);
    }
}
