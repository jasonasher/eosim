use crate::context::Context;
use serde::Serialize;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use tokio::sync::mpsc::Sender;

pub trait Report: Any {
    type Item;
}

struct ReportsDataContainer {
    report_item_handlers: HashMap<TypeId, Box<dyn Any>>,
}

impl ReportsDataContainer {
    fn new() -> ReportsDataContainer {
        ReportsDataContainer {
            report_item_handlers: HashMap::new(),
        }
    }
}

struct ReportItemHandler<T> {
    callback: Box<dyn FnMut(T)>,
}

impl<T> ReportItemHandler<T> {
    fn new(callback: impl FnMut(T) + 'static) -> ReportItemHandler<T> {
        ReportItemHandler {
            callback: Box::new(callback),
        }
    }
}

crate::context::define_plugin!(
    ReportsPlugin,
    ReportsDataContainer,
    ReportsDataContainer::new()
);

pub trait ReportsContext {
    fn set_report_item_handler<T: Report>(&mut self, callback: impl FnMut(T::Item) + 'static);

    fn release_report_item<T: Report>(&mut self, item: T::Item);
}

impl ReportsContext for Context {
    fn set_report_item_handler<T: Report>(&mut self, callback: impl FnMut(T::Item) + 'static) {
        let data_container = self.get_data_container_mut::<ReportsPlugin>();
        data_container.report_item_handlers.insert(
            TypeId::of::<T>(),
            Box::new(ReportItemHandler::new(callback)),
        );
    }

    fn release_report_item<T: Report>(&mut self, item: T::Item) {
        let data_container = self.get_data_container_mut::<ReportsPlugin>();
        match data_container
            .report_item_handlers
            .get_mut(&TypeId::of::<T>())
        {
            None => {} // Do nothing
            Some(handler) => {
                (handler
                    .downcast_mut::<ReportItemHandler<T::Item>>()
                    .unwrap()
                    .callback)(item);
            }
        };
    }
}

pub fn get_stdout_report_handler<T: Report>() -> impl FnMut(T::Item) + 'static
where
    T::Item: Serialize,
{
    let mut writer = csv::Writer::from_writer(io::stdout());
    move |item| {
        if let Err(e) = writer.serialize(item) {
            eprintln!("{}", e);
        }
    }
}

pub fn get_file_report_handler<T: Report>(file: File) -> impl FnMut(T::Item) + 'static
where
    T::Item: Serialize,
{
    let mut writer = csv::Writer::from_writer(file);
    move |item| {
        if let Err(e) = writer.serialize(item) {
            eprintln!("{}", e);
        }
    }
}

pub fn get_channel_report_handler<T: Report, S>(
    sender: Sender<(S, T::Item)>,
    id: S,
) -> impl FnMut(T::Item) + 'static
where
    T::Item: Serialize + Send + 'static,
    S: Serialize + Send + Copy + 'static,
{
    move |item| {
        if let Err(e) = sender.try_send((id, item)) {
            panic!("Failed to send item: {:?}", e);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::context::Context;
    use crate::reports::{
        get_channel_report_handler, get_file_report_handler, Report, ReportsContext,
    };
    use serde_derive::Serialize;
    use std::io::{Read, Seek};
    use tokio::sync::mpsc;
    use tokio::task;
    use tempfile::tempfile;

    crate::context::define_plugin!(TestReport, (), ());

    #[derive(Serialize)]
    struct TestItem {
        a: usize,
        b: bool,
    }

    impl Report for TestReport {
        type Item = TestItem;
    }

    #[derive(Serialize, Clone, Copy)]
    struct ReplicationData {
        scenario: usize,
        replication: usize,
    }

    fn release_report_items(context: &mut Context) {
        context.release_report_item::<TestReport>(TestItem { a: 23, b: true });
        context.release_report_item::<TestReport>(TestItem { a: 29, b: false });
        context.release_report_item::<TestReport>(TestItem { a: 2, b: true });
    }

    #[tokio::test]
    async fn test() {
        let mut context = Context::new();
        let output_file = tempfile().unwrap();
        context.set_report_item_handler::<TestReport>(get_file_report_handler::<TestReport>(
            output_file.try_clone().unwrap(),
        ));
        release_report_items(&mut context);
        // Drop context to flush output
        drop(context);

        let mut output_file = output_file.try_clone().unwrap();
        output_file.rewind().unwrap();
        let mut string = String::new();
        output_file.read_to_string(&mut string).unwrap();
        assert_eq!(string, "a,b\n23,true\n29,false\n2,true\n");

        // Threaded version
        let (sender, mut receiver) = mpsc::channel(100);
        let replication_data = ReplicationData {
            scenario: 0,
            replication: 0,
        };
        // Release reports in a thread
        let handle = task::spawn(async move {
            let mut context = Context::new();
            context.set_report_item_handler::<TestReport>(get_channel_report_handler::<
                TestReport,
                ReplicationData,
            >(sender, replication_data));
            release_report_items(&mut context);
        });

        let mut writer = csv::Writer::from_writer(vec![]);
        while let Some(item) = receiver.recv().await {
            writer.serialize(item).unwrap();
        }
        handle.await.unwrap();
        let output = String::from_utf8(writer.into_inner().unwrap()).unwrap();
        assert_eq!(
            output,
            "scenario,replication,a,b\n0,0,23,true\n0,0,29,false\n0,0,2,true\n"
        );
    }
}
