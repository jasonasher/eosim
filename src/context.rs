use std::{
    any::{Any, TypeId},
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
};

use derivative::Derivative;

pub trait Component: Any {
    fn init(context: &mut Context);
}

pub trait Plugin: Any {
    type DataContainer;

    fn get_data_container() -> Self::DataContainer;
}

#[macro_export]
macro_rules! define_plugin {
    ($plugin:ident, $data_container:ty, $default: expr) => {
        struct $plugin {}

        impl $crate::context::Plugin for $plugin {
            type DataContainer = $data_container;

            fn get_data_container() -> Self::DataContainer {
                $default
            }
        }
    };
}
pub use define_plugin;

pub struct PlanId {
    pub id: u64,
}

#[derive(Derivative)]
#[derivative(Eq, PartialEq, Debug)]
pub struct TimedPlan {
    pub time: f64,
    plan_id: u64,
    #[derivative(PartialEq = "ignore", Debug = "ignore")]
    pub callback: Box<dyn FnOnce(&mut Context)>,
}

impl Ord for TimedPlan {
    fn cmp(&self, other: &Self) -> Ordering {
        let time_ordering = self.time.partial_cmp(&other.time).unwrap().reverse();
        if time_ordering == Ordering::Equal {
            // Break time ties in order of plan id
            self.plan_id.cmp(&other.plan_id).reverse()
        } else {
            time_ordering
        }
    }
}

impl PartialOrd for TimedPlan {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
struct PlanQueue {
    queue: BinaryHeap<TimedPlan>,
    invalid_set: HashSet<u64>,
    plan_counter: u64,
}

impl PlanQueue {
    pub fn new() -> PlanQueue {
        PlanQueue {
            queue: BinaryHeap::new(),
            invalid_set: HashSet::new(),
            plan_counter: 0,
        }
    }

    pub fn add_plan(&mut self, time: f64, callback: impl FnOnce(&mut Context) + 'static) -> PlanId {
        // Add plan to queue and increment counter
        let plan_id = self.plan_counter;
        self.queue.push(TimedPlan {
            time,
            plan_id,
            callback: Box::new(callback),
        });
        self.plan_counter += 1;
        PlanId { id: plan_id }
    }

    pub fn cancel_plan(&mut self, id: PlanId) {
        self.invalid_set.insert(id.id);
    }

    pub fn get_next_timed_plan(&mut self) -> Option<TimedPlan> {
        loop {
            let next_timed_plan = self.queue.pop();
            match next_timed_plan {
                Some(timed_plan) => {
                    if self.invalid_set.contains(&timed_plan.plan_id) {
                        self.invalid_set.remove(&timed_plan.plan_id);
                    } else {
                        return Some(timed_plan);
                    }
                }
                None => {
                    return None;
                }
            }
        }
    }
}

type Callback = dyn FnOnce(&mut Context);
pub struct Context {
    plan_queue: PlanQueue,
    callback_queue: VecDeque<Box<Callback>>,
    plugin_data: HashMap<TypeId, Box<dyn Any>>,
    time: f64,
}

impl Context {
    pub fn new() -> Context {
        Context {
            plan_queue: PlanQueue::new(),
            callback_queue: VecDeque::new(),
            plugin_data: HashMap::new(),
            time: 0.0,
        }
    }

    pub fn add_plan(&mut self, time: f64, callback: impl FnOnce(&mut Context) + 'static) -> PlanId {
        // TODO: Handle invalid times (past, NAN, etc)
        self.plan_queue.add_plan(time, callback)
    }

    pub fn cancel_plan(&mut self, id: PlanId) {
        self.plan_queue.cancel_plan(id);
    }

    pub fn queue_callback(&mut self, callback: impl FnOnce(&mut Context) + 'static) {
        self.callback_queue.push_back(Box::new(callback));
    }

    fn add_plugin<T: Plugin>(&mut self) {
        self.plugin_data
            .insert(TypeId::of::<T>(), Box::new(T::get_data_container()));
    }

    pub fn get_data_container_mut<T: Plugin>(&mut self) -> &mut T::DataContainer {
        let type_id = &TypeId::of::<T>();
        if !self.plugin_data.contains_key(type_id) {
            self.add_plugin::<T>();
        }
        let data_container = self
            .plugin_data
            .get_mut(type_id)
            .unwrap()
            .downcast_mut::<T::DataContainer>();
        match data_container {
            Some(x) => x,
            None => panic!("Plugin data container of incorrect type"),
        }
    }

    pub fn get_data_container<T: Plugin>(&self) -> Option<&T::DataContainer> {
        let type_id = &TypeId::of::<T>();
        if !self.plugin_data.contains_key(type_id) {
            return None;
        }
        let data_container = self
            .plugin_data
            .get(type_id)
            .unwrap()
            .downcast_ref::<T::DataContainer>();
        match data_container {
            Some(x) => Some(x),
            None => panic!("Plugin data container of incorrect type"),
        }
    }

    pub fn get_time(&self) -> f64 {
        self.time
    }

    pub fn add_component<T: Component>(&mut self) {
        T::init(self);
    }

    pub fn execute(&mut self) {
        // Execute callbacks if there are any in the queue
        loop {
            let callback = self.callback_queue.pop_front();
            match callback {
                Some(callback) => callback(self),
                None => break,
            }
        }
        // Start plan loop
        loop {
            let timed_plan = self.plan_queue.get_next_timed_plan();
            match timed_plan {
                Some(timed_plan) => {
                    self.time = timed_plan.time;
                    (timed_plan.callback)(self);
                    loop {
                        let callback = self.callback_queue.pop_front();
                        match callback {
                            Some(callback) => callback(self),
                            None => break,
                        }
                    }
                }
                None => break,
            }
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    define_plugin!(ComponentA, u32, 0);

    impl ComponentA {
        fn increment_counter(context: &mut Context) {
            *(context.get_data_container_mut::<ComponentA>()) += 1;
        }
    }

    impl Component for ComponentA {
        fn init(context: &mut Context) {
            context.add_plan(1.0, Self::increment_counter);
        }
    }

    #[test]
    fn test_component_and_planning() {
        let mut context = Context::new();
        context.add_component::<ComponentA>();
        assert_eq!(context.get_time(), 0.0);
        assert_eq!(*context.get_data_container_mut::<ComponentA>(), 0);
        context.execute();
        assert_eq!(context.get_time(), 1.0);
        assert_eq!(*context.get_data_container_mut::<ComponentA>(), 1);
        let plan_to_cancel = context.add_plan(3.0, ComponentA::increment_counter);
        context.add_plan(2.0, ComponentA::increment_counter);
        context.cancel_plan(plan_to_cancel);
        context.execute();
        assert_eq!(context.get_time(), 2.0);
        assert_eq!(*context.get_data_container_mut::<ComponentA>(), 2);
    }
}
