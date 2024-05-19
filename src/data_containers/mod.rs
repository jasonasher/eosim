use std::any::Any;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::people::PersonId;

pub mod heterogeneous_container;

pub mod vector_heterogeneous_container;

pub mod indexset_person_container;

pub mod vector_person_container;

pub trait Property: Any {
    type Value: Any;
}

pub trait PropertyWithDefault: Any {
    type Value: Any + Copy;
    fn get_default() -> Self::Value;
}

impl<T: PropertyWithDefault> Property for T {
    type Value = T::Value;
}

pub trait PersonContainer {
    fn insert(&mut self, person_id: PersonId);

    fn remove(&mut self, person_id: &PersonId);

    fn contains(&self, person_id: &PersonId) -> bool;

    fn get_random(&self, rng: &mut impl Rng) -> Option<PersonId>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool;
}

pub fn test_add_remove_sample(container: &mut impl PersonContainer) {
    let mut rng = StdRng::seed_from_u64(8675309);
    for i in 0..4 {
        container.insert(PersonId::new(i));
    }

    assert_eq!(container.len(), 4);
    let sample = container.get_random(&mut rng).unwrap().id;
    assert!(sample <= 3);

    container.remove(&PersonId::new(0));
    assert_eq!(container.len(), 3);
    let sample = container.get_random(&mut rng).unwrap().id;
    assert!((1..=3).contains(&sample));

    container.remove(&PersonId::new(2));
    assert_eq!(container.len(), 2);
    let sample = container.get_random(&mut rng).unwrap().id;
    assert!((sample == 1) | (sample == 3));

    container.insert(PersonId::new(0));
    assert_eq!(container.len(), 3);
    let sample = container.get_random(&mut rng).unwrap().id;
    assert!((sample <= 1) | (sample == 3));

    container.remove(&PersonId::new(0));
    assert_eq!(container.len(), 2);
    let sample = container.get_random(&mut rng).unwrap().id;
    assert!((sample == 1) | (sample == 3));

    container.remove(&PersonId::new(3));
    assert_eq!(container.len(), 1);
    let sample = container.get_random(&mut rng).unwrap().id;
    assert_eq!(sample, 1);

    container.remove(&PersonId::new(1));
    assert_eq!(container.len(), 0);
    assert!(container.get_random(&mut rng).is_none());
}
