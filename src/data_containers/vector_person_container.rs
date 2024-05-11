use crate::data_containers::PersonContainer;
use crate::people::PersonId;
use core::slice;
use fxhash::FxBuildHasher;
use rand::Rng;
use std::collections::HashSet;

pub struct VecPersonContainer {
    people: Vec<PersonId>,
    invalid: HashSet<PersonId, FxBuildHasher>,
}

pub struct Iter<'a> {
    vec_iter: slice::Iter<'a, PersonId>,
    container: &'a VecPersonContainer,
}

impl<'a> Iterator for Iter<'a> {
    type Item = PersonId;

    fn next(&mut self) -> Option<PersonId> {
        let mut next_person_id = self.vec_iter.next();
        while next_person_id.is_some() {
            let person_id = next_person_id.unwrap();
            if self.container.invalid.contains(person_id) {
                next_person_id = self.vec_iter.next();
            } else {
                return Some(*person_id);
            }
        }
        None
    }
}

impl Default for VecPersonContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl VecPersonContainer {
    pub fn new() -> VecPersonContainer {
        VecPersonContainer {
            people: Vec::new(),
            invalid: HashSet::with_hasher(FxBuildHasher::default()),
        }
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            vec_iter: self.people.iter(),
            container: self,
        }
    }
}

impl<'a> IntoIterator for &'a VecPersonContainer {
    type Item = PersonId;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl PersonContainer for VecPersonContainer {
    fn insert(&mut self, person_id: PersonId) {
        if !self.invalid.is_empty() {
            let was_removed = self.invalid.remove(&person_id);
            if !was_removed {
                self.people.push(person_id);
            }
        } else {
            self.people.push(person_id);
        }
    }

    // TODO: Doesn't check the person is in the container, just marks as invalid
    fn remove(&mut self, person_id: &PersonId) {
        self.invalid.insert(*person_id);
        if (self.invalid.len() * 2 >= self.people.len()) & (self.people.len() >= 8) {
            let invalid = &mut self.invalid;
            self.people.retain(|x| !invalid.contains(x));
            //self.people = self.people.drain(..).filter(|x|{!invalid.contains(x)}).collect();
            invalid.clear();
        }
    }

    fn contains(&self, person_id: &PersonId) -> bool {
        if self.invalid.contains(person_id) {
            return false;
        }
        self.people.contains(person_id)
    }

    fn get_random(&self, rng: &mut impl Rng) -> Option<PersonId> {
        if self.is_empty() {
            return None;
        }
        let mut i;
        loop {
            i = rng.gen_range(0..self.people.len());
            if !self.invalid.contains(&self.people[i]) {
                return Some(self.people[i]);
            }
        }
    }

    fn len(&self) -> usize {
        // TODO: Invalid set may contain outside members if removed
        self.people.len() - self.invalid.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod test {
    use crate::data_containers::test_add_remove_sample;
    use crate::data_containers::vector_person_container::VecPersonContainer;

    #[test]
    fn new_remove_sample() {
        let mut container = VecPersonContainer::new();
        test_add_remove_sample(&mut container);
    }
}
