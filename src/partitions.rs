extern crate rand;

use crate::context::Context;
use crate::data_containers::indexset_person_container::IndexSetPersonContainer;
use crate::data_containers::PersonContainer;
use crate::people::{PeopleContext, PersonId};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

type ContextCallback = dyn FnOnce(&mut Context);
type LabelFunction<T> = dyn Fn(&Context, PersonId) -> T;
pub type PartitionUpdateCallbackProvider =
    dyn (Fn(&Context, PersonId) -> Box<dyn Fn(&mut Context)>);

pub trait Partition: Any {
    type LabelType: Any + Hash + Eq + Copy;

    fn get_update_callback_provider() -> impl (Fn(&Context, PersonId) -> Box<dyn Fn(&mut Context)>)
    where
        Self: Sized,
    {
        |context, person_id| {
            let current_label = context.get_partition_label::<Self>(person_id);
            Box::new(move |context| context.reevaluate_person::<Self>(person_id, current_label))
        }
    }
}

pub struct PartitionSpecification<T: Any + Hash + Eq> {
    pub(crate) label_function: Rc<LabelFunction<T>>,
    // Data to indicate when it needs to be updated
    pub(crate) registration_callback: Box<dyn FnOnce(&mut Context)>,
    pub(crate) deregistration_callback: Box<dyn FnOnce(&mut Context)>,
}

struct PartitionData<T: Any + Hash + Eq> {
    label_map: HashMap<T, IndexSetPersonContainer>,
    label_function: Rc<LabelFunction<T>>,
    deregistration_callback: Box<dyn FnOnce(&mut Context)>,
}

struct PartitionDataContainer {
    // Will map TypeId::of<P: PartitionKey> to PartitionData<P::Value>
    partition_map: HashMap<TypeId, Box<dyn Any>>,
}

crate::context::define_plugin!(
    PartitionPlugin,
    PartitionDataContainer,
    PartitionDataContainer {
        partition_map: HashMap::new(),
    }
);
pub struct PartitionBuilder<'a, P: Partition> {
    context: &'a mut Context,
    label_function: Option<Rc<LabelFunction<P::LabelType>>>,
    registration_callbacks: Vec<Box<ContextCallback>>,
    deregistration_callbacks: Vec<Box<ContextCallback>>,
}

impl<'a, P: Partition> PartitionBuilder<'a, P> {
    fn new(context: &'a mut Context) -> PartitionBuilder<'a, P> {
        PartitionBuilder {
            context,
            label_function: None,
            registration_callbacks: {
                let mut registration_callbacks: Vec<Box<ContextCallback>> = Vec::new();
                registration_callbacks.push(Box::new(|context: &mut Context| {
                    context.add_immediate_creation_callback::<P>(|context, person_id| {
                        context.handle_person_creation::<P>(person_id)
                    });
                }));
                registration_callbacks
            },
            deregistration_callbacks: {
                let mut deregistration_callbacks: Vec<Box<ContextCallback>> = Vec::new();
                deregistration_callbacks.push(Box::new(|context: &mut Context| {
                    context.remove_immediate_creation_callback::<P>();
                }));
                deregistration_callbacks
            },
        }
    }

    pub fn set_label_function(
        mut self,
        label_function: impl Fn(&Context, PersonId) -> P::LabelType + 'static,
    ) -> PartitionBuilder<'a, P> {
        self.label_function = Some(Rc::new(label_function));
        self
    }

    pub fn add_registration_callback(&mut self, callback: impl FnOnce(&mut Context) + 'static) {
        self.registration_callbacks.push(Box::new(callback));
    }

    pub fn add_deregistration_callback(&mut self, callback: impl FnOnce(&mut Context) + 'static) {
        self.deregistration_callbacks.push(Box::new(callback));
    }

    pub fn execute(self) {
        let (context, label_function, registration_callbacks, deregistration_callbacks) = (
            self.context,
            self.label_function,
            self.registration_callbacks,
            self.deregistration_callbacks,
        );
        let partition_specification = PartitionSpecification {
            label_function: label_function.expect("Label function not specified"),
            registration_callback: Box::new(move |context| {
                for callback in registration_callbacks {
                    (callback)(context)
                }
            }),
            deregistration_callback: Box::new(move |context| {
                for callback in deregistration_callbacks {
                    (callback)(context)
                }
            }),
        };
        context.add_partition_internal::<P>(partition_specification);
    }
}

pub trait PartitionContext {
    fn add_partition<P: Partition>(&mut self) -> PartitionBuilder<P>;
    fn remove_partition<P: Partition>(&mut self);
    fn get_partition_label<P: Partition>(&self, person_id: PersonId) -> P::LabelType;
    fn get_partition_cell<P: Partition>(
        &self,
        label: P::LabelType,
    ) -> Option<&IndexSetPersonContainer>;
}

impl PartitionContext for Context {
    fn add_partition<P: Partition>(&mut self) -> PartitionBuilder<P> {
        PartitionBuilder::new(self)
    }

    fn remove_partition<P: Partition>(&mut self) {
        let data_container = self.get_data_container_mut::<PartitionPlugin>();
        let partition_data = data_container.partition_map.remove(&TypeId::of::<P>());
        match partition_data {
            None => {
                panic!("Partition does not exist")
            }
            Some(partition_data) => {
                let partition_data = partition_data
                    .downcast::<PartitionData<P::LabelType>>()
                    .unwrap();
                (partition_data.deregistration_callback)(self);
            }
        }
    }

    fn get_partition_label<P: Partition>(&self, person_id: PersonId) -> P::LabelType {
        let data_container = self.get_data_container::<PartitionPlugin>().unwrap();
        let partition_data = data_container.partition_map.get(&TypeId::of::<P>());
        match partition_data {
            None => panic!("Partition not registered in Context"),
            Some(partition_data) => {
                let partition_data = partition_data
                    .downcast_ref::<PartitionData<P::LabelType>>()
                    .unwrap();
                (*partition_data.label_function)(self, person_id)
            }
        }
    }

    fn get_partition_cell<P: Partition>(
        &self,
        label: P::LabelType,
    ) -> Option<&IndexSetPersonContainer> {
        let data_container = self
            .get_data_container::<PartitionPlugin>()
            .expect("Partition plugin not loaded");
        let data = data_container
            .partition_map
            .get(&TypeId::of::<P>())
            .expect("Partition with specified key not loaded")
            .downcast_ref::<PartitionData<P::LabelType>>()
            .unwrap();
        data.label_map.get(&label)
    }
}

trait InternalPartitionContext {
    fn add_partition_internal<P: Partition>(
        &mut self,
        specification: PartitionSpecification<P::LabelType>,
    );

    fn reevaluate_person<P: Partition>(&mut self, person_id: PersonId, old_label: P::LabelType);

    fn handle_person_creation<P: Partition>(&mut self, person_id: PersonId);
}

impl InternalPartitionContext for Context {
    fn add_partition_internal<P: Partition>(
        &mut self,
        specification: PartitionSpecification<P::LabelType>,
    ) {
        // First build up the map of labels to PersonContainers
        let mut label_map: HashMap<P::LabelType, IndexSetPersonContainer> = HashMap::new();
        let maximum_person_id = self.get_maximum_person_id();
        if maximum_person_id.is_some() {
            // If there are people in the simulation, add them to the partition
            for i in 0..(maximum_person_id.unwrap().id + 1) {
                let person_id = PersonId::new(i);
                let label = (specification.label_function)(self, person_id);
                label_map
                    .entry(label)
                    .or_insert(IndexSetPersonContainer::new())
                    .insert(person_id)
            }
        }

        // Register for updates
        (specification.registration_callback)(self);

        // Store data
        let data_container = self.get_data_container_mut::<PartitionPlugin>();
        let partition_data = PartitionData {
            label_map,
            label_function: specification.label_function,
            deregistration_callback: specification.deregistration_callback,
        };
        data_container
            .partition_map
            .insert(TypeId::of::<P>(), Box::new(partition_data));
    }

    fn reevaluate_person<P: Partition>(&mut self, person_id: PersonId, old_label: P::LabelType) {
        let data_container = self.get_data_container::<PartitionPlugin>().unwrap();
        let partition_data = data_container.partition_map.get(&TypeId::of::<P>());
        if partition_data.is_none() {
            panic!("Partition not registered in Context");
        }
        let partition_data = partition_data
            .unwrap()
            .downcast_ref::<PartitionData<P::LabelType>>()
            .expect("Partition data of wrong type");
        let new_label = (*partition_data.label_function)(self, person_id);

        let data_container = self.get_data_container_mut::<PartitionPlugin>();
        let partition_data = data_container.partition_map.get_mut(&TypeId::of::<P>());
        match partition_data {
            None => panic!("Unreachable"),
            Some(partition_data) => {
                let partition_data = partition_data
                    .downcast_mut::<PartitionData<P::LabelType>>()
                    .unwrap();
                let old_label_people_container = partition_data.label_map.get_mut(&old_label);
                match old_label_people_container {
                    None => panic!("Old partition label is incorrect"),
                    Some(old_label_people_container) => {
                        if old_label_people_container.contains(&person_id) {
                            old_label_people_container.remove(&person_id);
                        } else {
                            panic!("Old partition label is incorrect");
                        }
                    }
                }
                let new_label_people_container = partition_data
                    .label_map
                    .entry(new_label)
                    .or_insert(IndexSetPersonContainer::new());
                new_label_people_container.insert(person_id);
            }
        }
    }

    fn handle_person_creation<P: Partition>(&mut self, person_id: PersonId) {
        let data_container = self.get_data_container::<PartitionPlugin>().unwrap();
        let partition_data = data_container.partition_map.get(&TypeId::of::<P>());
        if partition_data.is_none() {
            panic!("Partition not registered in Context");
        }
        let partition_data = partition_data
            .unwrap()
            .downcast_ref::<PartitionData<P::LabelType>>()
            .expect("Partition data of wrong type");
        let label = (*partition_data.label_function)(self, person_id);

        let data_container = self.get_data_container_mut::<PartitionPlugin>();
        let partition_data = data_container.partition_map.get_mut(&TypeId::of::<P>());
        match partition_data {
            None => panic!("Unreachable"),
            Some(partition_data) => {
                let partition_data = partition_data
                    .downcast_mut::<PartitionData<P::LabelType>>()
                    .unwrap();
                let label_people_container = partition_data
                    .label_map
                    .entry(label)
                    .or_insert_with(IndexSetPersonContainer::new);
                label_people_container.insert(person_id);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::context::Context;
    use crate::data_containers::PersonContainer;
    use crate::define_person_property;
    use crate::partitions::{Partition, PartitionContext};
    use crate::people::{PeopleContext, PersonId};
    use crate::person_properties::{PersonPropertyContext, PersonPropertyPartitionBuilder};
    use rand::prelude::StdRng;
    use rand::{Rng, SeedableRng};
    use std::collections::HashSet;

    define_person_property!(PropertyOne, u8, 0);
    define_person_property!(PropertyTwo, bool, false);

    struct PartitionOne {}

    impl Partition for PartitionOne {
        type LabelType = (u8, bool);
    }

    #[test]
    fn test() {
        let mut context = Context::new();
        let population = 10000;
        let n_to_change_properties = 1000;

        // Add a partition on PropertyOne
        context
            .add_partition::<PartitionOne>()
            .set_label_function(|context, person_id| {
                (
                    context.get_person_property_value::<PropertyOne>(person_id),
                    context.get_person_property_value::<PropertyTwo>(person_id),
                )
            })
            .add_person_property_sensitivity::<PropertyOne>()
            .add_person_property_sensitivity::<PropertyTwo>()
            .execute();

        for _ in 0..population {
            context.add_person().execute();
        }

        let zero_false_value_people = context
            .get_partition_cell::<PartitionOne>((0, false))
            .unwrap();
        assert_eq!(zero_false_value_people.len(), population);

        let mut people_to_change = HashSet::new();
        let mut rng = StdRng::seed_from_u64(8675309);
        loop {
            people_to_change.insert(PersonId::new(rng.gen_range(0..population)));
            if people_to_change.len() == n_to_change_properties {
                break;
            }
        }

        for person_id in &people_to_change {
            // Change one property or the other
            if rng.gen_bool(0.5) {
                context.set_person_property_value::<PropertyOne>(*person_id, 1);
            } else {
                context.set_person_property_value::<PropertyTwo>(*person_id, true);
            }
        }

        let zero_false_value_people = context
            .get_partition_cell::<PartitionOne>((0, false))
            .unwrap();
        assert_eq!(
            zero_false_value_people.len(),
            population - n_to_change_properties
        );
        assert!(context
            .get_partition_cell::<PartitionOne>((1, true))
            .is_none());
        let one_false_value_people = context
            .get_partition_cell::<PartitionOne>((1, false))
            .unwrap();
        let zero_true_value_people = context
            .get_partition_cell::<PartitionOne>((0, true))
            .unwrap();
        for person_id in &people_to_change {
            assert!(!zero_false_value_people.contains(person_id));
            assert!(
                one_false_value_people.contains(person_id)
                    ^ zero_true_value_people.contains(person_id)
            );
        }

        context.remove_partition::<PartitionOne>();
    }
}
