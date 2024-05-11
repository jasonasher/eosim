use crate::context::Context;

type CreatorFn<T> = dyn FnOnce(&mut Context) -> T;
type CreationCallback<T> = dyn FnOnce(&mut Context, T);

pub struct CreationBuilder<'a, T: Copy + Clone> {
    context: &'a mut Context,
    creator: Box<CreatorFn<T>>,
    callbacks: Vec<Box<CreationCallback<T>>>,
    finalizer: Box<CreationCallback<T>>,
}

impl<'a, T: Copy + Clone> CreationBuilder<'a, T> {
    pub fn new(
        context: &'a mut Context,
        creator: impl FnOnce(&mut Context) -> T + 'static,
        finalizer: impl FnOnce(&mut Context, T) + 'static,
    ) -> CreationBuilder<'a, T> {
        CreationBuilder {
            context,
            creator: Box::new(creator),
            callbacks: Vec::new(),
            finalizer: Box::new(finalizer),
        }
    }

    pub fn execute(self) -> T {
        let creation = (self.creator)(self.context);

        // Perform the builder callbacks (if any)
        for callback in self.callbacks {
            (callback)(self.context, creation)
        }

        (self.finalizer)(self.context, creation);

        creation
    }

    pub fn add_callback(&mut self, callback: impl FnOnce(&mut Context, T) + 'static) {
        self.callbacks.push(Box::new(callback));
    }
}
