use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::context::Context;
use crate::data_containers::vector_heterogeneous_container::VecDataContainer;
use crate::data_containers::PropertyWithDefault;
use crate::partitions::{Partition, PartitionBuilder, PartitionUpdateCallbackProvider};
use crate::people::{PersonBuilder, PersonId};

pub trait PersonProperty: PropertyWithDefault {}

static INDEX: Mutex<usize> = Mutex::new(0);

pub fn next(index: &AtomicUsize) -> usize {
    let mut guard = INDEX.lock().unwrap();
    if index.load(Ordering::SeqCst) == usize::MAX {
        index.store(*guard, Ordering::SeqCst);
        *guard += 1;
    }
    index.load(Ordering::Relaxed)
}

#[macro_export]
macro_rules! define_person_property {
    ($person_property:ident, $value:ty, $default: expr) => {
        pub struct $person_property {}

        impl $crate::data_containers::PropertyWithDefault for $person_property {
            type Value = $value;

            fn get_default() -> Self::Value {
                $default
            }

            fn index() -> usize {
                static INDEX: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(usize::MAX);
                let mut index = INDEX.load(std::sync::atomic::Ordering::Relaxed);
                if index == usize::MAX {
                    index = $crate::person_properties::next(&INDEX);
                }
                index
            }
        }

        impl $crate::person_properties::PersonProperty for $person_property {}
    };
}
pub use define_person_property;

#[macro_export]
macro_rules! define_person_property_from_enum {
    ($person_property:ty, $default: expr) => {
        impl $crate::data_containers::PropertyWithDefault for $person_property {
            type Value = $person_property;

            fn get_default() -> Self::Value {
                $default
            }

            fn index() -> usize {
                static INDEX: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(usize::MAX);
                let mut index = INDEX.load(std::sync::atomic::Ordering::Relaxed);
                if index == usize::MAX {
                    index = $crate::context::next(&INDEX);
                }
                index
            }
        }

        impl $crate::person_properties::PersonProperty for $person_property {}

        impl Copy for $person_property {}

        impl Clone for $person_property {
            fn clone(&self) -> Self {
                *self
            }
        }
    };
}
pub use define_person_property_from_enum;

struct PersonPropertyDataContainer {
    person_property_container: VecDataContainer,
    person_property_change_callbacks: HashMap<TypeId, Box<dyn Any>>,
    partition_update_callback_providers:
        HashMap<TypeId, HashMap<TypeId, Box<PartitionUpdateCallbackProvider>>>,
}

crate::context::define_plugin!(
    PersonPropertyPlugin,
    PersonPropertyDataContainer,
    PersonPropertyDataContainer {
        person_property_container: VecDataContainer::new(),
        person_property_change_callbacks: HashMap::new(),
        partition_update_callback_providers: HashMap::new(),
    }
);

type ContextCallback = dyn FnOnce(&mut Context);
type PersonPropertyChangeCallback<T> = dyn Fn(&mut Context, PersonId, T);

pub trait PersonPropertyContext {
    fn get_person_property_value<T: PersonProperty>(&self, person_id: PersonId) -> T::Value;

    fn set_person_property_value<T: PersonProperty>(
        &mut self,
        person_id: PersonId,
        value: T::Value,
    );

    fn observe_person_property_changes<T: PersonProperty>(
        &mut self,
        callback: impl Fn(&mut Context, PersonId, T::Value) + 'static,
    );

    fn add_person_property_partition_callback<T: PersonProperty, K: Partition>(
        &mut self,
        provider: impl (Fn(&Context, PersonId) -> Box<dyn Fn(&mut Context)>) + 'static,
    );

    fn remove_person_property_partition_callback<T: PersonProperty, K: Partition>(&mut self);
}

impl PersonPropertyContext for Context {
    fn get_person_property_value<T: PersonProperty>(&self, person_id: PersonId) -> T::Value {
        let data_container = self.get_data_container::<PersonPropertyPlugin>();
        match data_container {
            None => T::get_default(),
            Some(data_container) => data_container
                .person_property_container
                .get_value::<T>(person_id.id),
        }
    }

    fn set_person_property_value<T: PersonProperty>(
        &mut self,
        person_id: PersonId,
        value: T::Value,
    ) {
        let mut callbacks_to_add = Vec::<Box<ContextCallback>>::new();
        let mut partition_callbacks = Vec::new();
        if let Some(data_container) = self.get_data_container::<PersonPropertyPlugin>() {
            // Observation callbacks
            let callback_vec = data_container
                .person_property_change_callbacks
                .get(&TypeId::of::<T>());
            if callback_vec.is_some() {
                let callback_vec: &Vec<Rc<PersonPropertyChangeCallback<T::Value>>> =
                    callback_vec.unwrap().downcast_ref().unwrap();
                if !callback_vec.is_empty() {
                    let current_value = data_container
                        .person_property_container
                        .get_value::<T>(person_id.id);
                    for callback in callback_vec {
                        let internal_callback = Rc::clone(callback);
                        callbacks_to_add.push(Box::new(move |context| {
                            internal_callback(context, person_id, current_value)
                        }));
                    }
                }
            }
            // Partition callbacks
            let partition_callback_map = data_container
                .partition_update_callback_providers
                .get(&TypeId::of::<T>());
            if partition_callback_map.is_some() {
                let partition_callback_map = partition_callback_map.unwrap();
                for entry in partition_callback_map {
                    let partition_update_callback = (entry.1)(self, person_id);
                    partition_callbacks.push(partition_update_callback);
                }
            }
        }

        for callback in callbacks_to_add {
            self.queue_callback(callback);
        }

        let data_container = self.get_data_container_mut::<PersonPropertyPlugin>();
        data_container
            .person_property_container
            .set_value::<T>(person_id.id, value);

        // Update partitions
        for partition_callback in partition_callbacks {
            partition_callback(self)
        }
    }

    fn observe_person_property_changes<T: PersonProperty>(
        &mut self,
        callback: impl Fn(&mut Context, PersonId, T::Value) + 'static,
    ) {
        let data_container = self.get_data_container_mut::<PersonPropertyPlugin>();
        let callback_vec = data_container
            .person_property_change_callbacks
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::<Vec<Rc<PersonPropertyChangeCallback<T::Value>>>>::default());
        let callback_vec: &mut Vec<Rc<PersonPropertyChangeCallback<T::Value>>> =
            callback_vec.downcast_mut().unwrap();
        callback_vec.push(Rc::new(callback));
    }

    fn add_person_property_partition_callback<T: PersonProperty, K: Partition>(
        &mut self,
        provider: impl (Fn(&Context, PersonId) -> Box<dyn Fn(&mut Context)>) + 'static,
    ) {
        let data_container = self.get_data_container_mut::<PersonPropertyPlugin>();
        let provider_map = data_container
            .partition_update_callback_providers
            .entry(TypeId::of::<T>())
            .or_default();
        provider_map.insert(TypeId::of::<K>(), Box::new(provider));
    }

    fn remove_person_property_partition_callback<T: PersonProperty, K: Partition>(&mut self) {
        let data_container = self.get_data_container_mut::<PersonPropertyPlugin>();
        let provider_map = data_container
            .partition_update_callback_providers
            .get_mut(&TypeId::of::<T>());
        if let Some(provider_map) = provider_map {
            provider_map.remove(&TypeId::of::<K>());
        }
    }
}

pub trait PersonPropertiesPersonBuilder<'a> {
    fn set_person_property<T: PersonProperty>(self, value: T::Value) -> PersonBuilder<'a>;
}

impl<'a> PersonPropertiesPersonBuilder<'a> for PersonBuilder<'a> {
    fn set_person_property<T: PersonProperty>(mut self, value: T::Value) -> PersonBuilder<'a> {
        self.add_callback(move |context, person_id: PersonId| {
            let data_container = context.get_data_container_mut::<PersonPropertyPlugin>();
            data_container
                .person_property_container
                .set_value::<T>(person_id.id, value);
        });
        self
    }
}

pub trait PersonPropertyPartitionBuilder<'a, P: Partition> {
    fn add_person_property_sensitivity<T: PersonProperty>(self) -> PartitionBuilder<'a, P>;
}

impl<'a, P: Partition> PersonPropertyPartitionBuilder<'a, P> for PartitionBuilder<'a, P> {
    fn add_person_property_sensitivity<T: PersonProperty>(mut self) -> PartitionBuilder<'a, P> {
        self.add_registration_callback(|context| {
            context
                .add_person_property_partition_callback::<T, P>(P::get_update_callback_provider());
        });
        self.add_deregistration_callback(|context| {
            context.remove_person_property_partition_callback::<T, P>();
        });
        self
    }
}

#[cfg(test)]
mod test {
    use crate::context::{Component, Context};
    use crate::people::PeopleContext;
    use crate::person_properties::{
        PersonId, PersonPropertiesPersonBuilder, PersonPropertyContext,
    };

    define_person_property!(PropertyOne, usize, 0);

    enum PropertyTwo {
        A,
        B,
    }
    define_person_property_from_enum!(PropertyTwo, PropertyTwo::A);

    #[test]
    fn test() {
        let mut context = Context::new();
        assert_eq!(
            context.get_person_property_value::<PropertyOne>(PersonId { id: 0 }),
            0
        );

        context.set_person_property_value::<PropertyOne>(PersonId { id: 1 }, 1);
        context.set_person_property_value::<PropertyTwo>(PersonId { id: 1 }, PropertyTwo::B);

        assert_eq!(
            context.get_person_property_value::<PropertyOne>(PersonId { id: 0 }),
            0
        );
        assert_eq!(
            context.get_person_property_value::<PropertyOne>(PersonId { id: 1 }),
            1
        );
        assert!(matches!(
            context.get_person_property_value::<PropertyTwo>(PersonId { id: 0 }),
            PropertyTwo::A
        ));
        assert!(matches!(
            context.get_person_property_value::<PropertyTwo>(PersonId { id: 1 }),
            PropertyTwo::B
        ));
    }

    struct ComponentA {}
    impl ComponentA {
        fn handle_person_property_value_assignment(
            context: &mut Context,
            person_id: PersonId,
            _value: usize,
        ) {
            context.set_person_property_value::<PropertyTwo>(person_id, PropertyTwo::B);
        }

        fn set_person_0_property_one_to_1(context: &mut Context) {
            context.set_person_property_value::<PropertyOne>(PersonId { id: 0 }, 1);
        }
    }
    impl Component for ComponentA {
        fn init(_context: &mut Context) {
            unimplemented!()
        }
    }

    #[test]
    fn test_observation() {
        let mut context = Context::new();
        context.observe_person_property_changes::<PropertyOne>(
            ComponentA::handle_person_property_value_assignment,
        );
        assert!(matches!(
            context.get_person_property_value::<PropertyTwo>(PersonId { id: 0 }),
            PropertyTwo::A
        ));
        context.add_plan(1.0, ComponentA::set_person_0_property_one_to_1);
        context.execute();
        assert!(matches!(
            context.get_person_property_value::<PropertyTwo>(PersonId { id: 0 }),
            PropertyTwo::B
        ));
        context.add_plan(2.0, |context| {
            context.set_person_property_value::<PropertyOne>(PersonId { id: 1 }, 1)
        });
        context.execute();
        assert!(matches!(
            context.get_person_property_value::<PropertyTwo>(PersonId { id: 1 }),
            PropertyTwo::B
        ));
    }

    #[test]
    fn test_creation() {
        let mut context = Context::new();
        let person = context
            .add_person()
            .set_person_property::<PropertyOne>(1)
            .set_person_property::<PropertyTwo>(PropertyTwo::B)
            .execute();
        assert_eq!(context.get_person_property_value::<PropertyOne>(person), 1);
        assert!(matches!(
            context.get_person_property_value::<PropertyTwo>(person),
            PropertyTwo::B
        ));
        assert_eq!(context.get_maximum_person_id(), Some(PersonId::new(0)));
    }
}
