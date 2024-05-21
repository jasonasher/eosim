use std::{
    any::Any,
    cmp::Ordering as CmpOrdering,
    collections::{BinaryHeap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

use derivative::Derivative;

pub trait Component: Any {
    fn init(context: &mut Context);
}

pub trait Plugin: Any {
    type DataContainer;

    fn get_data_container() -> Self::DataContainer;

    fn index() -> usize;
}

static INDEX: Mutex<usize> = Mutex::new(0);

pub fn next(index: &AtomicUsize) -> usize {
    let mut guard = INDEX.lock().unwrap();
    if index.load(Ordering::SeqCst) == usize::MAX {
        index.store(*guard, Ordering::SeqCst);
        *guard += 1;
    }
    index.load(Ordering::Relaxed)
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

            fn index() -> usize {
                static INDEX: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(usize::MAX);
                let mut index = INDEX.load(std::sync::atomic::Ordering::Relaxed);
                if index == usize::MAX {
                    index = $crate::context::next(&INDEX);
                }
                index
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
    fn cmp(&self, other: &Self) -> CmpOrdering {
        let time_ordering = self.time.partial_cmp(&other.time).unwrap().reverse();
        if time_ordering == CmpOrdering::Equal {
            // Break time ties in order of plan id
            self.plan_id.cmp(&other.plan_id)
        } else {
            time_ordering
        }
    }
}

impl PartialOrd for TimedPlan {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
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
    plugin_data: Vec<Option<Box<dyn Any>>>,
    time: f64,
}

impl Context {
    pub fn new() -> Context {
        Context {
            plan_queue: PlanQueue::new(),
            callback_queue: VecDeque::new(),
            plugin_data: Vec::new(),
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

    pub fn get_data_container_mut<T: Plugin>(&mut self) -> &mut T::DataContainer {
        let index = T::index();
        let plugin_data = &mut self.plugin_data;
        if index >= plugin_data.len() {
            plugin_data.resize_with(index + 1, || None);
        }
        if plugin_data[index].is_none() {
            plugin_data[index] = Some(Box::new(T::get_data_container()));
        }

        plugin_data[index]
            .as_mut()
            .unwrap()
            .downcast_mut::<T::DataContainer>()
            .unwrap()
    }

    pub fn get_data_container<T: Plugin>(&self) -> Option<&T::DataContainer> {
        let index = T::index();
        match self.plugin_data.get(index) {
            Some(None) => None,
            Some(data_container) => Some(
                data_container
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<T::DataContainer>()
                    .unwrap(),
            ),
            None => None,
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
