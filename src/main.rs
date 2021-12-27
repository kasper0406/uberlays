mod plot;
mod iracing;
mod overlay;

#[macro_use] extern crate log;
extern crate env_logger;

use log::Level;

use std::sync::mpsc::{ channel, Receiver, Sender };
use std::thread;
use std::time::{ Instant };

use plot::PlotOverlay;
use iracing::{ Update, Telemetry };

fn main() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let (sender, receiver) = channel();

    let samples_per_second = 60;
    let data_producer = thread::spawn(move || {
        let start = Instant::now();
        // TODO(knielsen): Terminate this thread!
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
        }
    });

    let overlay = PlotOverlay::new(receiver);
    overlay.create_window(800.0, 150.0);

    data_producer.join();
}
