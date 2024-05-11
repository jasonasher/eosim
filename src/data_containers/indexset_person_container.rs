use super::PersonContainer;
use crate::people::PersonId;
use fxhash::FxBuildHasher;
use indexmap::set::IndexSet;
use rand::Rng;

pub struct IndexSetPersonContainer {
    people: IndexSet<PersonId, FxBuildHasher>,
}

impl Default for IndexSetPersonContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexSetPersonContainer {
    pub fn new() -> IndexSetPersonContainer {
        IndexSetPersonContainer {
            people: IndexSet::with_hasher(FxBuildHasher::default()),
        }
    }

    pub fn with_capacity(n: usize) -> IndexSetPersonContainer {
        IndexSetPersonContainer {
            people: IndexSet::with_capacity_and_hasher(n, FxBuildHasher::default()),
        }
    }
}

impl PersonContainer for IndexSetPersonContainer {
    fn insert(&mut self, person_id: PersonId) {
        self.people.insert(person_id);
    }

    fn remove(&mut self, person_id: &PersonId) {
        self.people.swap_remove(person_id);
    }

    fn contains(&self, person_id: &PersonId) -> bool {
        self.people.contains(person_id)
    }

    fn get_random(&self, rng: &mut impl Rng) -> Option<PersonId> {
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

    fn len(&self) -> usize {
        self.people.len()
    }

    fn is_empty(&self) -> bool {
        self.people.is_empty()
    }
}

#[cfg(test)]
mod test {
    use super::IndexSetPersonContainer;
    use crate::data_containers::test_add_remove_sample;

    #[test]
    fn new_remove_sample() {
        let mut container = IndexSetPersonContainer::new();
        test_add_remove_sample(&mut container);
    }
}
