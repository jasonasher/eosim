use std::any::Any;
use std::rc::Rc;

use crate::data_containers::Property;

pub struct HeterogeneousContainer {
    data: Vec<Option<Rc<dyn Any>>>,
}

impl Default for HeterogeneousContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl HeterogeneousContainer {
    pub fn new() -> HeterogeneousContainer {
        HeterogeneousContainer { data: Vec::new() }
    }

    pub fn set_value<K: Property>(&mut self, value: K::Value) {
        let property_index = K::index();
        if property_index >= self.data.len() {
            self.data.resize_with(property_index + 1, || None)
        }
        self.data[property_index] = Some(Rc::new(value));
    }

    pub fn get_value<K: Property>(&self) -> Option<&K::Value> {
        let property_index = K::index();
        if property_index >= self.data.len() {
            return None;
        }
        self.data[property_index]
            .as_ref()
            .map(|boxed_value| boxed_value.downcast_ref::<K::Value>().unwrap())
    }

    pub fn get_rc_value<K: Property>(&self) -> Option<Rc<K::Value>> {
        let property_index = K::index();
        if property_index >= self.data.len() {
            return None;
        }
        self.data[property_index]
            .as_ref()
            .map(|boxed_value| Rc::clone(boxed_value).downcast::<K::Value>().unwrap())
    }
}

mod tests {
    use super::*;

    struct KeyOne {}
    impl Property for KeyOne {
        type Value = usize;
        fn index() -> usize {
            0
        }
    }

    struct KeyTwo {}
    impl Property for KeyTwo {
        type Value = bool;
        fn index() -> usize {
            1
        }
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
