mod iracing;
mod overlay;
mod plot;
mod head2head;
mod track;

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

use iracing::data_collector;
use iracing::data_collector::IracingConnection;
use iracing::data_collector::IracingConnectionError;
use iracing::data_collector::IracingValue;
use iracing::data_collector::DataHeader;

fn extract_value<T>(telemetry: &[IracingValue], header: Option<(usize, &DataHeader)>, extractor: Box<dyn Fn(&IracingValue) -> T>) -> T {
    if let Some((idx, _)) = header {
        (extractor)(&telemetry[idx])
    } else {
        (extractor)(&IracingValue::Unknown)
    }
}

fn main() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    unsafe {
        SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS);
    }

    let (sender, receiver) = async_std::channel::unbounded();

    /*
    let data_producer = task::spawn(async move {
        loop {
            sender.send(Update::Telemetry(Telemetry {
                timestamp: Instant::now(),
                throttle: 0.0,
                brake: 0.0,
                gear: 1,
                velocity: 0.0,
                deltas: vec![0.364, 14.340, -2.423, -23.42],
            })).await.unwrap();

            thread::sleep(std::time::Duration::from_millis(50));
        }
    }); */

    let data_producer = task::spawn(async move {
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
                .find(|(_, header)| header.name == "Throttle");
        let brake_header = headers.iter().enumerate()
                .find(|(_, header)| header.name == "Brake");
        let positions_header = headers.iter().enumerate()
                .find(|(_, header)| header.name == "CarIdxLapDistPct");

        let mut packages = 0;
        while let Some(package) = connection.next().await {
            packages += 1;
            if packages % 60 == 0 {
                info!["Package count: {}", packages];
            }

            match package {
                data_collector::Update::Telemetry(telemetry) => {
                    let throttle = extract_value(&telemetry, throttle_header, Box::new(|val| match val {
                        IracingValue::Float(throttle) => throttle.clone(),
                        _ => 0.0
                    }));
                    let brake = extract_value(&telemetry, brake_header, Box::new(|val| match val {
                        IracingValue::Float(brake) => brake.clone(),
                        _ => 0.0
                    }));
                    let positions = extract_value(&telemetry, positions_header, Box::new(|val| match val {
                        IracingValue::FloatVector(positions) => positions.clone(),
                        _ => vec![]
                    }));

                    let timestamp = Instant::now();
                    sender.send(Update::Telemetry(Telemetry {
                        timestamp,
                        throttle,
                        brake,
                        gear: 1,
                        velocity: 0.0,
                        deltas: vec![0.364, 14.340, -2.423, -23.42],
                        positions,
                    })).await.unwrap();
                },
                data_collector::Update::SessionInfo(session_info) => {
                    info!["Session info: {}", session_info]
                }
            }
        }
    });

    let overlays = Overlays::new(receiver);
    overlays.start_event_loop();

    task::block_on(data_producer);
}
