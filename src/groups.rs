use crate::context::{Context, Plugin};
use crate::data_containers::vector_person_container::VecPersonContainer;
use crate::data_containers::{PersonContainer, PropertyWithDefault};
use crate::people::PersonId;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use tinyset::SetUsize;

pub trait GroupType: Any + Hash + Eq + PartialEq {}

// TODO: Implement Group properties
pub trait GroupProperty: PropertyWithDefault {
    type Group: GroupType;
}

#[derive(Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct GroupId<T: GroupType> {
    pub id: usize,
    // Marker to say this group id is associated with T (but does not own it)
    pub group_type: PhantomData<*const T>,
}

impl<T: GroupType> Clone for GroupId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: GroupType> Copy for GroupId<T> {}

impl<T: GroupType> GroupId<T> {
    pub fn new(id: usize) -> GroupId<T> {
        GroupId {
            id,
            group_type: PhantomData,
        }
    }
}

struct GroupsDataContainer {
    // Stores for each GroupType T an GroupId<T> that represents the maximum id of that type (if it exists)
    max_group_id: HashMap<TypeId, Box<dyn Any>>,
    // Stores for each GroupType a vector by PersonId of the set of groups by id that the person belongs to
    person_to_group_map: HashMap<TypeId, Vec<SetUsize>>,
    // Stores for each GroupType a vector by GroupId of the set of people by id in that group
    group_to_person_map: HashMap<TypeId, Vec<VecPersonContainer>>,
    // TODO: Group properties (by group type)
}

struct GroupsPlugin {}

impl Plugin for GroupsPlugin {
    type DataContainer = GroupsDataContainer;

    fn get_data_container() -> GroupsDataContainer {
        GroupsDataContainer {
            max_group_id: HashMap::new(),
            person_to_group_map: HashMap::new(),
            group_to_person_map: HashMap::new(),
        }
    }
}

pub trait GroupsContext {
    fn add_group<T: GroupType>(&mut self) -> GroupId<T>;

    fn get_maximum_group_id<T: GroupType>(&self) -> Option<GroupId<T>>;

    fn add_person_to_group<T: GroupType>(&mut self, person_id: PersonId, group_id: GroupId<T>);

    fn get_group_members<T: GroupType>(&self, group_id: GroupId<T>) -> Option<&VecPersonContainer>;

    fn get_groups_for_person<T: GroupType>(&self, person_id: PersonId) -> Vec<GroupId<T>>;
}

impl GroupsContext for Context {
    fn add_group<T: GroupType>(&mut self) -> GroupId<T> {
        let data_container = self.get_data_container_mut::<GroupsPlugin>();
        let group_type_id = TypeId::of::<T>();
        let max_group_id = data_container.max_group_id.get_mut(&group_type_id);
        return match max_group_id {
            None => {
                let new_id = GroupId::<T>::new(0);
                data_container
                    .max_group_id
                    .insert(group_type_id, Box::new(new_id));
                new_id
            }
            Some(max_group_id) => {
                let max_group_id: &mut GroupId<T> = max_group_id.downcast_mut().unwrap();
                max_group_id.id += 1;
                *max_group_id
            }
        };
    }

    fn get_maximum_group_id<T: GroupType>(&self) -> Option<GroupId<T>> {
        let data_container = self.get_data_container::<GroupsPlugin>();
        return match data_container {
            None => None,
            Some(data_container) => {
                let max_group_id = data_container.max_group_id.get(&TypeId::of::<T>());
                return max_group_id.map(|max_group_id| *max_group_id.downcast_ref().unwrap());
            }
        };
    }

    fn add_person_to_group<T: GroupType>(&mut self, person_id: PersonId, group_id: GroupId<T>) {
        let data_container = self.get_data_container_mut::<GroupsPlugin>();
        let group_type_id = TypeId::of::<T>();
        // Add person to group to person map
        let group_people_vec = data_container
            .group_to_person_map
            .entry(group_type_id)
            .or_insert_with(|| {
                let mut new_vec = Vec::with_capacity(group_id.id);
                new_vec.resize_with(group_id.id + 1, VecPersonContainer::new);
                new_vec
            });
        if group_id.id >= group_people_vec.len() {
            group_people_vec.resize_with(group_id.id + 1, VecPersonContainer::new);
        }
        let group_people = &mut group_people_vec[group_id.id];
        group_people.insert(person_id);
        // Add group to person to group map
        let people_group_vec = data_container
            .person_to_group_map
            .entry(group_type_id)
            .or_insert_with(|| {
                let mut new_vec = Vec::with_capacity(person_id.id);
                new_vec.resize_with(person_id.id + 1, SetUsize::new);
                new_vec
            });
        if person_id.id >= people_group_vec.len() {
            people_group_vec.resize_with(person_id.id + 1, SetUsize::new);
        }
        let person_groups = &mut people_group_vec[person_id.id];
        person_groups.insert(group_id.id);
    }

    fn get_group_members<T: GroupType>(&self, group_id: GroupId<T>) -> Option<&VecPersonContainer> {
        let data_container = self.get_data_container::<GroupsPlugin>();
        match data_container {
            None => panic!("Group plugin hasn't loaded"),
            Some(data_container) => {
                let group_people_vec = data_container.group_to_person_map.get(&TypeId::of::<T>());
                match group_people_vec {
                    None => panic!("Group id is invalid"),
                    Some(group_people_vec) => {
                        // TODO Check that group id has been issued
                        if group_id.id >= group_people_vec.len() {
                            return None;
                        }
                        Some(&group_people_vec[group_id.id])
                    }
                }
            }
        }
    }

    fn get_groups_for_person<T: GroupType>(&self, person_id: PersonId) -> Vec<GroupId<T>> {
        let data_container = self.get_data_container::<GroupsPlugin>();
        return match data_container {
            None => Vec::new(),
            Some(data_container) => {
                let people_group_vec = data_container.person_to_group_map.get(&TypeId::of::<T>());
                return match people_group_vec {
                    None => Vec::new(),
                    Some(people_group_vec) => {
                        if person_id.id >= people_group_vec.len() {
                            return Vec::new();
                        }
                        let mut group_id_vec = Vec::new();
                        let group_ids = &people_group_vec[person_id.id];
                        for group_id in group_ids.iter() {
                            group_id_vec.push(GroupId::new(group_id));
                        }
                        return group_id_vec;
                    }
                };
            }
        };
    }
}

#[cfg(test)]
mod test {
    use crate::context::Context;
    use crate::data_containers::PersonContainer;
    use crate::groups::{GroupType, GroupsContext};
    use crate::people::PersonId;

    #[derive(Eq, PartialEq, Hash)]
    struct GroupTypeOne {}
    impl GroupType for GroupTypeOne {}

    #[derive(Eq, PartialEq, Hash)]
    struct GroupTypeTwo {}
    impl GroupType for GroupTypeTwo {}

    #[test]
    fn test() {
        let mut context = Context::new();
        // Should be no groups of either type
        assert!(context.get_maximum_group_id::<GroupTypeOne>().is_none());
        assert!(context.get_maximum_group_id::<GroupTypeTwo>().is_none());

        // Add a group of type one
        let group_type_one_id = context.add_group::<GroupTypeOne>();
        match context.get_maximum_group_id::<GroupTypeOne>() {
            None => panic!("Error"),
            Some(group_id) => {
                assert!(group_type_one_id.eq(&group_id))
            }
        }
        assert!(context.get_maximum_group_id::<GroupTypeTwo>().is_none());
        // Add some people to the first group
        context.add_person_to_group(PersonId::new(0), group_type_one_id);
        context.add_person_to_group(PersonId::new(2), group_type_one_id);
        context.add_person_to_group(PersonId::new(3), group_type_one_id);
        let group_type_one_members = context.get_group_members(group_type_one_id);
        match group_type_one_members {
            None => panic!("Error"),
            Some(members) => {
                assert_eq!(members.len(), 3);
                assert!(members.contains(&PersonId::new(0)));
                assert!(members.contains(&PersonId::new(2)));
                assert!(members.contains(&PersonId::new(3)));
            }
        }
        // Check people's group memberships
        let group_ids = context.get_groups_for_person::<GroupTypeOne>(PersonId::new(0));
        assert_eq!(group_ids.len(), 1);
        assert!(group_ids.contains(&group_type_one_id));
        let group_ids = context.get_groups_for_person::<GroupTypeOne>(PersonId::new(1));
        assert_eq!(group_ids.len(), 0);
        let group_ids = context.get_groups_for_person::<GroupTypeOne>(PersonId::new(2));
        assert_eq!(group_ids.len(), 1);
        assert!(group_ids.contains(&group_type_one_id));
        let group_ids = context.get_groups_for_person::<GroupTypeOne>(PersonId::new(3));
        assert_eq!(group_ids.len(), 1);
        assert!(group_ids.contains(&group_type_one_id));

        // Add a group of type two
        context.add_group::<GroupTypeTwo>();
        let group_type_two_id = context.add_group::<GroupTypeTwo>();
        match context.get_maximum_group_id::<GroupTypeTwo>() {
            None => panic!("Error"),
            Some(group_id) => {
                assert!(group_type_two_id.eq(&group_id))
            }
        }
        // Add some people to the second group
        context.add_person_to_group(PersonId::new(1), group_type_two_id);
        context.add_person_to_group(PersonId::new(2), group_type_two_id);
        let group_type_two_members = context.get_group_members(group_type_two_id);
        match group_type_two_members {
            None => panic!("Error"),
            Some(members) => {
                assert_eq!(members.len(), 2);
                assert!(members.contains(&PersonId::new(1)));
                assert!(members.contains(&PersonId::new(2)));
            }
        }
        // Check people's group memberships
        let group_ids = context.get_groups_for_person::<GroupTypeTwo>(PersonId::new(0));
        assert_eq!(group_ids.len(), 0);
        let group_ids = context.get_groups_for_person::<GroupTypeTwo>(PersonId::new(1));
        assert_eq!(group_ids.len(), 1);
        assert!(group_ids.contains(&group_type_two_id));
        let group_ids = context.get_groups_for_person::<GroupTypeTwo>(PersonId::new(2));
        assert_eq!(group_ids.len(), 1);
        assert!(group_ids.contains(&group_type_two_id));
        let group_ids = context.get_groups_for_person::<GroupTypeTwo>(PersonId::new(3));
        assert_eq!(group_ids.len(), 0);
    }
}
