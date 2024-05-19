use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;

use crate::context::{Context, Plugin};
use crate::data_containers::heterogeneous_container::HeterogeneousContainer;
use crate::data_containers::Property;

// TODO: Decide if we want default global property values
pub trait GlobalProperty: Property {}

#[macro_export]
macro_rules! define_global_property {
    ($global_property:ident, $value:ty) => {
        pub struct $global_property {}

        impl Property for $global_property {
            type Value = $value;
        }

        impl GlobalProperty for $global_property {}
    };
}
pub use define_global_property;

struct GlobalPropertyDataContainer {
    global_property_container: HeterogeneousContainer,
    global_property_change_callbacks: HashMap<TypeId, Box<dyn Any>>,
}

type GlobalPropertyChangeCallback<T> = dyn Fn(&mut Context, Option<&T>);
struct GlobalPropertyPlugin {}

impl Plugin for GlobalPropertyPlugin {
    type DataContainer = GlobalPropertyDataContainer;

    fn get_data_container() -> Self::DataContainer {
        GlobalPropertyDataContainer {
            global_property_container: HeterogeneousContainer::new(),
            global_property_change_callbacks: HashMap::new(),
        }
    }
}

pub trait GlobalPropertyContext {
    fn get_global_property_value<T: GlobalProperty>(&self) -> Option<&T::Value>;

    fn set_global_property_value<T: GlobalProperty>(&mut self, value: T::Value);

    fn observe_global_property_changes<T: GlobalProperty>(
        &mut self,
        callback: impl Fn(&mut Context, Option<&T::Value>) + 'static,
    );
}

impl GlobalPropertyContext for Context {
    fn get_global_property_value<T: GlobalProperty>(&self) -> Option<&T::Value> {
        let data_container = self.get_data_container::<GlobalPropertyPlugin>();
        return match data_container {
            None => None,
            Some(data_container) => data_container.global_property_container.get_value::<T>(),
        };
    }

    fn set_global_property_value<T: GlobalProperty>(&mut self, value: T::Value) {
        // First look for callbacks
        let mut callbacks_to_add = Vec::<Box<dyn FnOnce(&mut Context)>>::new();
        let data_container = self.get_data_container::<GlobalPropertyPlugin>();
        if data_container.is_some() {
            let data_container = data_container.unwrap();
            // Observation callbacks
            let callback_vec = data_container
                .global_property_change_callbacks
                .get(&TypeId::of::<T>());
            if callback_vec.is_some() {
                let callback_vec: &Vec<Rc<GlobalPropertyChangeCallback<T::Value>>> =
                    callback_vec.unwrap().downcast_ref().unwrap();
                if !callback_vec.is_empty() {
                    let current_value =
                        data_container.global_property_container.get_rc_value::<T>();
                    for callback in callback_vec {
                        let internal_callback = Rc::clone(callback);
                        match &current_value {
                            None => callbacks_to_add
                                .push(Box::new(move |context| (*internal_callback)(context, None))),
                            Some(current_value) => {
                                let current_value_clone = Rc::clone(current_value);
                                callbacks_to_add.push(Box::new(move |context| {
                                    (*internal_callback)(context, Some(&*current_value_clone))
                                }));
                            }
                        }
                    }
                }
            }
        }
        for callback in callbacks_to_add {
            self.queue_callback(callback);
        }
        let data_container = self.get_data_container_mut::<GlobalPropertyPlugin>();
        data_container
            .global_property_container
            .set_value::<T>(value);
    }

    fn observe_global_property_changes<T: GlobalProperty>(
        &mut self,
        callback: impl Fn(&mut Context, Option<&T::Value>) + 'static,
    ) {
        let data_container = self.get_data_container_mut::<GlobalPropertyPlugin>();
        let callback_vec = data_container
            .global_property_change_callbacks
            .entry(TypeId::of::<T>())
            .or_insert_with(|| {
                let new_vec = Vec::<Rc<GlobalPropertyChangeCallback<T::Value>>>::new();
                Box::new(new_vec)
            });
        let callback_vec: &mut Vec<Rc<GlobalPropertyChangeCallback<T::Value>>> =
            callback_vec.downcast_mut().unwrap();
        callback_vec.push(Rc::new(callback));
    }
}

#[cfg(test)]
mod test {
    use crate::context::Context;
    use crate::data_containers::Property;
    use crate::global_properties::{GlobalProperty, GlobalPropertyContext};

    define_global_property!(PropertyA, usize);

    #[derive(Copy, Clone)]
    pub struct PropertyBValues {
        old_a_value: usize,
        number_of_calls: usize,
    }

    define_global_property!(PropertyB, PropertyBValues);

    #[test]
    fn test() {
        let mut context = Context::new();
        context.set_global_property_value::<PropertyB>(PropertyBValues {
            old_a_value: 0,
            number_of_calls: 0,
        });
        context.set_global_property_value::<PropertyA>(1);
        context.observe_global_property_changes::<PropertyA>(|context, old_value| {
            let current_b_value = context.get_global_property_value::<PropertyB>().unwrap();
            let number_of_calls = current_b_value.number_of_calls;
            context.set_global_property_value::<PropertyB>(PropertyBValues {
                old_a_value: *old_value.unwrap(),
                number_of_calls: number_of_calls + 1,
            });
        });
        context.set_global_property_value::<PropertyA>(2);
        context.execute();
        let current_b_value = context.get_global_property_value::<PropertyB>().unwrap();
        assert_eq!(current_b_value.number_of_calls, 1);
        assert_eq!(current_b_value.old_a_value, 1);

        context.set_global_property_value::<PropertyA>(3);
        context.execute();
        let current_b_value = context.get_global_property_value::<PropertyB>().unwrap();
        assert_eq!(current_b_value.number_of_calls, 2);
        assert_eq!(current_b_value.old_a_value, 2);
    }
}
