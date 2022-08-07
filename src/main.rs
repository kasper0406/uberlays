mod iracing;
mod overlay;
mod plot;
mod head2head;
mod track;

#[macro_use] extern crate log;
extern crate env_logger;
extern crate yaml_rust;

use windows::{
    Win32::System::Threading::*,
};

use async_std::task;
use overlay::Overlays;

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    unsafe {
        SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS);
    }

    let (sender, receiver) = async_std::channel::unbounded();

    // let data_producer = iracing::data_producer::TestTask::new(sender);
    let data_producer = iracing::data_producer::IracingTask::new(sender);
    let data_producer_thread = task::spawn(async {
        data_producer.execute().await
    });

    let overlays = Overlays::new(receiver);
    overlays.start_event_loop();

    task::block_on(data_producer_thread);
}
