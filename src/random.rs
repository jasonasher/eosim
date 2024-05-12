use crate::context::{Context, Plugin};
use rand::SeedableRng;
use std::any::{Any, TypeId};
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;

pub trait RandomId: Any {
    type RngType: SeedableRng;
    fn seed_offset() -> u64;
}

struct RandomHolder {
    rng: Box<dyn Any>,
    reseed: bool,
}

struct RandomData {
    base_seed: u64,
    random_holders: RefCell<HashMap<TypeId, RandomHolder>>,
}

struct RandomPlugin {}

impl Plugin for RandomPlugin {
    type DataContainer = RandomData;

    fn get_data_container() -> Self::DataContainer {
        RandomData {
            base_seed: 0,
            random_holders: RefCell::new(HashMap::new()),
        }
    }
}

pub trait RandomContext {
    fn set_base_random_seed(&mut self, base_seed: u64);

    fn get_rng<R: RandomId>(&self) -> RefMut<'_, R::RngType>;
}

impl RandomContext for Context {
    fn set_base_random_seed(&mut self, base_seed: u64) {
        let data_container = self.get_data_container_mut::<RandomPlugin>();
        data_container.base_seed = base_seed;
        let mut random_holders = data_container.random_holders.try_borrow_mut().unwrap();
        for random_holder in random_holders.values_mut() {
            random_holder.reseed = true;
        }
    }

    fn get_rng<R: RandomId>(&self) -> RefMut<'_, R::RngType> {
        let data_container = self.get_data_container::<RandomPlugin>().unwrap();
        let base_seed = data_container.base_seed;
        let random_holders = data_container.random_holders.try_borrow_mut().unwrap();
        let mut random_holder = RefMut::map(random_holders, |random_holders| {
            random_holders
                .entry(TypeId::of::<R>())
                .or_insert_with(|| RandomHolder {
                    rng: Box::new(R::RngType::seed_from_u64(base_seed + R::seed_offset())),
                    reseed: false,
                })
        });
        if random_holder.reseed {
            random_holder.rng = Box::new(R::RngType::seed_from_u64(base_seed + R::seed_offset()));
            random_holder.reseed = false;
        }
        RefMut::map(random_holder, |random_holder| {
            random_holder.rng.downcast_mut::<R::RngType>().unwrap()
        })
    }
}

#[cfg(test)]
mod test {
    use crate::context::Context;
    use crate::random::{RandomContext, RandomId};
    use rand::rngs::StdRng;
    use rand::RngCore;

    struct RandomIdOne {}
    impl RandomId for RandomIdOne {
        type RngType = StdRng;

        fn seed_offset() -> u64 {
            1
        }
    }

    struct RandomIdTwo {}
    impl RandomId for RandomIdTwo {
        type RngType = StdRng;

        fn seed_offset() -> u64 {
            2
        }
    }

    #[test]
    fn test() {
        let mut context = Context::new();
        context.set_base_random_seed(8675309);
        let mut rng_one = context.get_rng::<RandomIdOne>();
        let rng_one_sample_1 = rng_one.next_u64();
        let rng_one_sample_2 = rng_one.next_u64();
        drop(rng_one);
        let mut rng_two = context.get_rng::<RandomIdTwo>();
        let rng_two_sample_1 = rng_two.next_u64();
        let rng_two_sample_2 = rng_two.next_u64();
        drop(rng_two);
        assert_ne!(rng_one_sample_1, rng_one_sample_2);
        assert_ne!(rng_two_sample_1, rng_two_sample_2);
        assert_ne!(rng_one_sample_1, rng_two_sample_1);
        assert_ne!(rng_one_sample_2, rng_two_sample_2);
        assert_ne!(rng_one_sample_1, rng_two_sample_2);
        assert_ne!(rng_two_sample_1, rng_one_sample_2);

        context.set_base_random_seed(8675309);
        let mut rng_one = context.get_rng::<RandomIdOne>();
        assert_eq!(rng_one_sample_1, rng_one.next_u64());
        assert_eq!(rng_one_sample_2, rng_one.next_u64());
        drop(rng_one);
        let mut rng_two = context.get_rng::<RandomIdTwo>();
        assert_eq!(rng_two_sample_1, rng_two.next_u64());
        assert_eq!(rng_two_sample_2, rng_two.next_u64());
    }
}
