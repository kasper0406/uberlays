mod iracing;
mod overlay;
mod plot;
mod head2head;

#[macro_use] extern crate log;
extern crate env_logger;

use windows::{
    Win32::Foundation::*,
    Win32::System::Threading::*,
};

use log::Level;

use std::thread;
use std::time::{ Instant };

use tokio::sync::broadcast;

use overlay::Overlays;
use iracing::{ Update, Telemetry };

fn main() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    unsafe {
        SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS);
    }

    let (sender, receiver) = broadcast::channel(16);

    let samples_per_second = 60;
    let data_producer = thread::spawn(move || {
        let start = Instant::now();
        // TODO(knielsen): Terminate this thread!
        /*
        loop {
            let time = Instant::now();
            sender.send(Update::Telemetry(Telemetry {
                timestamp: time,
                throttle: (1f32 + time.duration_since(start).as_secs_f32().sin()) / 2f32,
                r#break: 0.0,
                gear: 1,
                velocity: 250.0,
                deltas: vec![0.364, 14.340, -2.423, -23.42],
            }));

            thread::sleep_ms(1000 / samples_per_second);
        } */

        iracing::data_collector::iracing();
    });

    let overlays = Overlays::new(receiver);
    overlays.start_event_loop();

    data_producer.join();
}
