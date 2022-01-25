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

use async_std::task;
use async_std::channel::Receiver;
use async_std::stream::StreamExt;

use overlay::Overlays;
use iracing::{ Update, Telemetry };

use iracing::data_collector::IracingConnection;
use iracing::data_collector::IracingConnectionError;
use iracing::data_collector::IracingValue;

fn main() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    unsafe {
        SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS);
    }

    let (sender, receiver) = async_std::channel::unbounded();

    let data_producer = task::spawn(async move {
        let start = Instant::now();

        let mut maybe_connection: Option<IracingConnection> = None;
        loop {
            match IracingConnection::new() {
                Ok(new_connection) => {
                    maybe_connection = Some(new_connection);
                    break;
                },
                Err(IracingConnectionError::NotRunning) => {
                    info!("iRacing not detected. Retrying!");
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        }
        let mut connection = maybe_connection.unwrap();

        info!("Established connection to iRacing");

        let headers = connection.headers();
        // info!["Headers: {:?}", headers];
        let throttle_header = headers.iter().enumerate()
                                    .find(|(idx, header)| header.name == "Throttle")
                                    .unwrap();
        let break_header = headers.iter().enumerate()
                                    .find(|(idx, header)| header.name == "Brake")
                                    .unwrap();

        let mut ticks = 0;
        while let Some(package) = connection.next().await {
            ticks += 1;
            if ticks % 60 == 0 {
                info!["Tick count: {}", ticks];
            }

            let throttle = if let IracingValue::Float(received_throttle) = package[throttle_header.0] {
                received_throttle
            } else { 0.0 };

            let brake = if let IracingValue::Float(received_break) = package[break_header.0] {
                received_break
            } else { 0.0 };

            let timestamp = Instant::now();
            sender.send(Update::Telemetry(Telemetry {
                timestamp,
                throttle,
                brake,
                gear: 1,
                velocity: 0.0,
                deltas: vec![0.364, 14.340, -2.423, -23.42],
            })).await.unwrap();
        }
    });

    let overlays = Overlays::new(receiver);
    overlays.start_event_loop();

    task::block_on(data_producer);
}
