use crate::context::Context;
use crate::creation::CreationBuilder;
use crate::data_containers::vector_heterogeneous_container::VecDataContainer;
use crate::data_containers::PropertyWithDefault;
use crate::people::{PersonBuilder, PersonId};

#[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Debug)]
pub struct RegionId {
    pub id: usize,
}

impl RegionId {
    pub fn new(id: usize) -> RegionId {
        RegionId { id }
    }
}

pub trait RegionProperty: PropertyWithDefault {}

#[macro_export]
macro_rules! define_region_property {
    ($region_property:ident, $value:ty, $default: expr) => {
        pub struct $region_property {}

        impl $crate::data_containers::PropertyWithDefault for $region_property {
            type Value = $value;

            fn get_default() -> Self::Value {
                $default
            }
        }

        impl $crate::regions::RegionProperty for $region_property {}
    };
}
pub use define_region_property;

fn create_region(context: &mut Context) -> RegionId {
    let data_container = context.get_data_container_mut::<RegionsPlugin>();
    let region_id = match data_container.max_region_id {
        None => RegionId::new(0),
        Some(max_region_id) => RegionId::new(max_region_id.id + 1),
    };
    data_container.max_region_id = Some(region_id);
    region_id
}

pub trait RegionPropertiesCreationBuilder<'a> {
    fn set_region_property<T: RegionProperty>(
        self,
        value: T::Value,
    ) -> CreationBuilder<'a, RegionId>;
}

impl<'a> RegionPropertiesCreationBuilder<'a> for CreationBuilder<'a, RegionId> {
    fn set_region_property<T: RegionProperty>(
        mut self,
        value: T::Value,
    ) -> CreationBuilder<'a, RegionId> {
        self.add_callback(move |context, region_id: RegionId| {
            let data_container = context.get_data_container_mut::<RegionsPlugin>();
            data_container
                .region_property_container
                .set_value::<T>(region_id.id, value);
        });
        self
    }
}

struct RegionsDataContainer {
    max_region_id: Option<RegionId>,
    // Maps person by PersonId to RegionId
    region_map: Vec<RegionId>,
    region_property_container: VecDataContainer,
}

crate::context::define_plugin!(
    RegionsPlugin,
    RegionsDataContainer,
    RegionsDataContainer {
        max_region_id: None,
        region_map: Vec::new(),
        region_property_container: VecDataContainer::new()
    }
);

#[allow(clippy::manual_non_exhaustive)]
pub struct PersonRegionChangeEvent {
    pub person_id: PersonId,
    pub old_region_id: RegionId,
    _private: (),
}
impl Copy for PersonRegionChangeEvent {}
impl Clone for PersonRegionChangeEvent {
    fn clone(&self) -> Self {
        *self
    }
}

pub trait RegionsContext {
    fn add_region(&mut self) -> CreationBuilder<RegionId>;

    fn get_maximum_region_id(&self) -> Option<RegionId>;

    fn get_person_region(&self, person_id: PersonId) -> RegionId;

    fn set_person_region(&mut self, person_id: PersonId, region_id: RegionId);

    fn get_region_property_value<T: RegionProperty>(&self, region_id: RegionId) -> T::Value;

    fn set_region_property_value<T: RegionProperty>(
        &mut self,
        region_id: RegionId,
        value: T::Value,
    );

    fn observe_person_region_changes(
        &mut self,
        callback: impl Fn(&mut Context, PersonId, RegionId) + 'static,
    );
}

impl RegionsContext for Context {
    fn add_region(&mut self) -> CreationBuilder<RegionId> {
        CreationBuilder::new(self, create_region, |_context, _region_id| {})
    }

    fn get_maximum_region_id(&self) -> Option<RegionId> {
        let data_container = self.get_data_container::<RegionsPlugin>();
        match data_container {
            None => None,
            Some(data_container) => data_container.max_region_id,
        }
    }

    fn get_person_region(&self, person_id: PersonId) -> RegionId {
        let data_container = self
            .get_data_container::<RegionsPlugin>()
            .expect("Regions plugin hasn't been loaded.");
        *data_container
            .region_map
            .get(person_id.id)
            .expect("Person hasn't been assigned a region")
    }

    fn set_person_region(&mut self, person_id: PersonId, region_id: RegionId) {
        let data_container = self.get_data_container_mut::<RegionsPlugin>();

        if person_id.id >= data_container.region_map.len() {
            panic!("Person hasn't been assigned a region")
        }

        // Build event signaling region has changed
        let current_region = data_container.region_map[person_id.id];
        let change_event = PersonRegionChangeEvent {
            person_id,
            old_region_id: current_region,
            _private: (),
        };

        // Record new region
        data_container.region_map[person_id.id] = region_id;

        // Release event
        self.release_event(change_event);
    }

    fn get_region_property_value<T: RegionProperty>(&self, region_id: RegionId) -> T::Value {
        let data_container = self.get_data_container::<RegionsPlugin>();
        match data_container {
            None => T::get_default(),
            Some(data_container) => data_container
                .region_property_container
                .get_value::<T>(region_id.id),
        }
    }

    fn set_region_property_value<T: RegionProperty>(
        &mut self,
        region_id: RegionId,
        value: T::Value,
    ) {
        let data_container = self.get_data_container_mut::<RegionsPlugin>();
        data_container
            .region_property_container
            .set_value::<T>(region_id.id, value);
    }

    fn observe_person_region_changes(
        &mut self,
        callback: impl Fn(&mut Context, PersonId, RegionId) + 'static,
    ) {
        self.subscribe_to_event::<PersonRegionChangeEvent>(move |context, event| {
            callback(context, event.person_id, event.old_region_id);
        })
    }
}

pub trait RegionsPersonBuilder<'a> {
    fn set_region(self, region_id: RegionId) -> PersonBuilder<'a>;
}

impl<'a> RegionsPersonBuilder<'a> for PersonBuilder<'a> {
    fn set_region(mut self, region_id: RegionId) -> PersonBuilder<'a> {
        // TODO: Validation to require that this method be called (because a region must be specified)
        self.add_callback(move |context, person_id: PersonId| {
            let data_container = context.get_data_container_mut::<RegionsPlugin>();
            if person_id.id != data_container.region_map.len() {
                panic!("Expecting sequential person ids");
            }
            data_container.region_map.push(region_id);
        });
        self
    }
}

#[cfg(test)]
mod test {
    use crate::context::{Component, Context};
    use crate::data_containers::PersonContainer;
    use crate::partitions::{Partition, PartitionContext, RegionsPartitionBuilder};
    use crate::people::PeopleContext;
    use crate::regions::{
        RegionId, RegionPropertiesCreationBuilder, RegionsContext, RegionsPersonBuilder,
    };

    define_region_property!(RegionPropertyA, f64, 0.0);

    #[test]
    fn test() {
        let mut context = Context::new();
        assert_eq!(context.get_maximum_person_id(), None);
        assert_eq!(context.get_maximum_region_id(), None);

        // Add some regions
        let region_zero = context.add_region().execute();
        assert_eq!(context.get_maximum_region_id(), Some(region_zero));
        assert_eq!(
            context.get_region_property_value::<RegionPropertyA>(region_zero),
            0.0
        );
        let my_float = 43.0 / 7.0;
        let region_one = context
            .add_region()
            .set_region_property::<RegionPropertyA>(my_float)
            .execute();
        assert_eq!(
            context.get_region_property_value::<RegionPropertyA>(region_one),
            my_float
        );

        // Add some people
        let new_person = context.add_person().set_region(region_zero).execute();
        assert_eq!(context.get_maximum_person_id(), Some(new_person));
        assert_eq!(context.get_person_region(new_person), region_zero);
        let new_person = context.add_person().set_region(region_one).execute();
        assert_eq!(context.get_maximum_person_id(), Some(new_person));
        assert_eq!(context.get_person_region(new_person), region_one);
    }

    crate::context::define_plugin!(ComponentOne, Option<RegionId>, None);

    impl Component for ComponentOne {
        fn init(context: &mut Context) {
            context.observe_person_region_changes(|context, _person_id, region_id| {
                *context.get_data_container_mut::<ComponentOne>() = Some(region_id);
            })
        }
    }

    struct PartitionOneKey {}
    impl Partition for PartitionOneKey {
        type LabelType = RegionId;
    }

    #[test]
    fn test_observe() {
        let mut context = Context::new();

        context.add_component::<ComponentOne>();
        assert!(context.get_data_container_mut::<ComponentOne>().is_none());

        context
            .add_partition::<PartitionOneKey>()
            .set_label_function(|context, person_id| context.get_person_region(person_id))
            .add_region_sensitivity(|_region, new_region| new_region)
            .execute();

        let region_zero = context.add_region().execute();
        let person_id = context.add_person().set_region(region_zero).execute();
        context.execute();
        assert!(context.get_data_container_mut::<ComponentOne>().is_none());
        let cell = context.get_partition_cell::<PartitionOneKey>(region_zero);
        assert!(cell.is_some());
        assert_eq!(cell.unwrap().len(), 1);
        assert!(cell.unwrap().contains(&person_id));

        let new_region = context.add_region().execute();
        context.set_person_region(person_id, new_region);
        context.execute();
        assert_eq!(
            *context.get_data_container_mut::<ComponentOne>(),
            Some(region_zero)
        );
        let cell = context.get_partition_cell::<PartitionOneKey>(new_region);
        assert!(cell.is_some());
        assert_eq!(cell.unwrap().len(), 1);
        assert!(cell.unwrap().contains(&person_id));

        context.set_person_region(person_id, region_zero);
        context.execute();
        assert_eq!(
            *context.get_data_container_mut::<ComponentOne>(),
            Some(new_region)
        );
        let cell = context.get_partition_cell::<PartitionOneKey>(region_zero);
        assert!(cell.is_some());
        assert_eq!(cell.unwrap().len(), 1);
        assert!(cell.unwrap().contains(&person_id));
    }
}
