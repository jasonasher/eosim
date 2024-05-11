use crate::context::{Context, Plugin};
use crate::creation::CreationBuilder;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Hash, Eq, PartialEq, Clone, Copy, Ord, PartialOrd, Debug)]
pub struct PersonId {
    pub id: usize,
}

impl PersonId {
    pub fn new(id: usize) -> PersonId {
        PersonId { id }
    }
}

pub type PersonBuilder<'a> = CreationBuilder<'a, PersonId>;

fn create_person(context: &mut Context) -> PersonId {
    // Add a person to the simulation
    let data_container = context.get_data_container_mut::<PeoplePlugin>();
    let person_id = match data_container.max_person_id {
        None => PersonId::new(0),
        Some(max_person_id) => PersonId::new(max_person_id.id + 1),
    };
    data_container.max_person_id = Some(person_id);
    person_id
}

fn finalize_person_creation(context: &mut Context, person_id: PersonId) {
    let data_container = context.get_data_container::<PeoplePlugin>().unwrap();
    // Collect the immediate execution callbacks
    let immediate_callbacks = Rc::clone(&data_container.creation_immediate_callbacks);
    // Collect the observation callbacks
    let creation_observers = Rc::clone(&data_container.creation_observers);

    // Perform the immediate execution callbacks
    for callback in immediate_callbacks.borrow().values() {
        (callback)(context, person_id);
    }

    // Add the observation callbacks
    for callback in creation_observers.borrow().values() {
        let internal_callback = Rc::clone(callback);
        context.add_callback(move |context| (internal_callback)(context, person_id));
    }
}

type PersonCreationCallback = dyn Fn(&mut Context, PersonId);
struct PeopleDataContainer {
    max_person_id: Option<PersonId>,
    creation_immediate_callbacks: Rc<RefCell<HashMap<TypeId, Rc<PersonCreationCallback>>>>,
    creation_observers: Rc<RefCell<HashMap<TypeId, Rc<PersonCreationCallback>>>>,
}

struct PeoplePlugin {}

impl Plugin for PeoplePlugin {
    type DataContainer = PeopleDataContainer;

    fn get_data_container() -> Self::DataContainer {
        PeopleDataContainer {
            max_person_id: None,
            creation_immediate_callbacks: Rc::new(RefCell::new(HashMap::new())),
            creation_observers: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}

pub trait PeopleContext {
    fn add_person(&mut self) -> PersonBuilder;

    fn get_maximum_person_id(&self) -> Option<PersonId>;

    fn observe_person_creation<T: Any>(
        &mut self,
        callback: impl Fn(&mut Context, PersonId) + 'static,
    );

    fn ignore_person_creation<T: Any>(&mut self);

    fn add_immediate_creation_callback<T: Any>(
        &mut self,
        callback: impl Fn(&mut Context, PersonId) + 'static,
    );

    fn remove_immediate_creation_callback<T: Any>(&mut self);
}

impl PeopleContext for Context {
    fn add_person(&mut self) -> PersonBuilder {
        PersonBuilder::new(self, create_person, finalize_person_creation)
    }

    fn get_maximum_person_id(&self) -> Option<PersonId> {
        let data_container = self.get_data_container::<PeoplePlugin>();
        match data_container {
            None => None,
            Some(data_container) => data_container.max_person_id,
        }
    }

    fn observe_person_creation<T: Any>(
        &mut self,
        callback: impl Fn(&mut Context, PersonId) + 'static,
    ) {
        let data_container = self.get_data_container_mut::<PeoplePlugin>();
        data_container
            .creation_observers
            .borrow_mut()
            .insert(TypeId::of::<T>(), Rc::new(callback));
    }

    fn ignore_person_creation<T: Any>(&mut self) {
        let data_container = self.get_data_container_mut::<PeoplePlugin>();
        data_container
            .creation_observers
            .borrow_mut()
            .remove(&TypeId::of::<T>());
    }

    fn add_immediate_creation_callback<T: Any>(
        &mut self,
        callback: impl Fn(&mut Context, PersonId) + 'static,
    ) {
        let data_container = self.get_data_container_mut::<PeoplePlugin>();
        data_container
            .creation_immediate_callbacks
            .borrow_mut()
            .insert(TypeId::of::<T>(), Rc::new(callback));
    }

    fn remove_immediate_creation_callback<T: Any>(&mut self) {
        let data_container = self.get_data_container_mut::<PeoplePlugin>();
        data_container
            .creation_immediate_callbacks
            .borrow_mut()
            .remove(&TypeId::of::<T>());
    }
}

#[cfg(test)]
mod tests {
    use crate::context::{Context, Plugin};

    use super::{PeopleContext, PersonId};

    struct PluginA {}
    impl Plugin for PluginA {
        type DataContainer = Option<PersonId>;

        fn get_data_container() -> Self::DataContainer {
            None
        }
    }

    #[test]
    fn test_add_person() {
        let mut context = Context::new();

        context.observe_person_creation::<PluginA>(|context, person_id| {
            *context.get_data_container_mut::<PluginA>() = Some(person_id)
        });

        // Test observation callback (queued)
        assert_eq!(context.get_data_container_mut::<PluginA>(), &None);
        context.add_person().execute();
        assert_eq!(context.get_data_container_mut::<PluginA>(), &None);
        // Trigger callback in queue
        context.execute();
        assert_eq!(
            context.get_data_container::<PluginA>().unwrap(),
            &context.get_maximum_person_id()
        );

        // Test immediate callback
        context.add_immediate_creation_callback::<PluginA>(|context, person_id| {
            *context.get_data_container_mut::<PluginA>() = Some(person_id)
        });
        context.add_person().execute();
        assert_eq!(
            context.get_data_container::<PluginA>().unwrap(),
            &context.get_maximum_person_id()
        );
    }
}
