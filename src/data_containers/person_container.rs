use rand::Rng;

use crate::people::PersonId;
use fxhash::FxBuildHasher;
use indexmap::set::IndexSet;

pub struct PersonContainer {
    people: IndexSet<PersonId, FxBuildHasher>,
}

impl Default for PersonContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl PersonContainer {
    pub fn new() -> PersonContainer {
        PersonContainer {
            people: IndexSet::with_hasher(FxBuildHasher::default()),
        }
    }

    pub fn with_capacity(n: usize) -> PersonContainer {
        PersonContainer {
            people: IndexSet::with_capacity_and_hasher(n, FxBuildHasher::default()),
        }
    }

    pub fn insert(&mut self, person_id: PersonId) {
        self.people.insert(person_id);
    }

    pub fn remove(&mut self, person_id: &PersonId) {
        self.people.swap_remove(person_id);
    }

    pub fn contains(&self, person_id: &PersonId) -> bool {
        self.people.contains(person_id)
    }

    pub fn get_random(&self, rng: &mut impl Rng) -> Option<PersonId> {
        if self.people.is_empty() {
            return None;
        }
        Some(
            *self
                .people
                .get_index(rng.gen_range(0..self.people.len()))
                .unwrap(),
        )
    }

    pub fn get_nth(&self, n: usize) -> PersonId {
        if n >= self.len() {
            panic!("Out of bounds of person container");
        }
        *self.people.get_index(n).unwrap()
    }

    pub fn len(&self) -> usize {
        self.people.len()
    }

    pub fn is_empty(&self) -> bool {
        self.people.is_empty()
    }
}

#[cfg(test)]
mod test {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use crate::people::PersonId;

    use super::PersonContainer;

    #[test]
    fn new_remove_sample() {
        let mut container = PersonContainer::new();
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
        assert!((1 <= sample) & (sample <= 3));

        container.remove(&PersonId::new(2));
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
}
