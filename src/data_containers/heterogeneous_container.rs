use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;
use std::rc::Rc;

use crate::data_containers::Property;

pub struct HeterogeneousContainer {
    map: HashMap<TypeId, Rc<dyn Any>>,
}

impl Default for HeterogeneousContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl HeterogeneousContainer {
    pub fn new() -> HeterogeneousContainer {
        HeterogeneousContainer {
            map: HashMap::new(),
        }
    }

    pub fn set_value<K: Property>(&mut self, value: K::Value) {
        self.map.insert(TypeId::of::<K>(), Rc::new(value));
    }

    pub fn get_value<K: Property>(&self) -> Option<&K::Value> {
        match self.map.get(&TypeId::of::<K>()) {
            None => None,
            Some(boxed_value) => boxed_value.downcast_ref::<K::Value>(),
        }
    }

    pub fn get_rc_value<K: Property>(&self) -> Option<Rc<K::Value>> {
        self.map
            .get(&TypeId::of::<K>())
            .map(|boxed_value| Rc::clone(boxed_value).downcast::<K::Value>().unwrap())
    }
}

mod tests {
    use super::*;

    struct KeyOne {}
    impl Property for KeyOne {
        type Value = usize;
    }

    struct KeyTwo {}
    impl Property for KeyTwo {
        type Value = bool;
    }

    #[test]
    fn test_container() {
        let mut container = HeterogeneousContainer::new();

        container.set_value::<KeyOne>(1);
        assert_eq!(*container.get_value::<KeyOne>().unwrap(), 1);
        assert_eq!(container.get_value::<KeyTwo>(), None);

        container.set_value::<KeyTwo>(false);
        assert_eq!(*container.get_value::<KeyOne>().unwrap(), 1);
        assert_eq!(*container.get_value::<KeyTwo>().unwrap(), false);

        container.set_value::<KeyOne>(2);
        assert_eq!(*container.get_value::<KeyOne>().unwrap(), 2);
    }
}
