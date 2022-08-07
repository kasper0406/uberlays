use std::time::{ Instant, Duration };

use async_std::task;
use async_std::channel::Sender;
use async_std::stream::StreamExt;

use crate::iracing::{ Update, Telemetry, SessionInfo, TrackSpec, DriverInfo };
use crate::iracing::data_collector;
use crate::iracing::data_collector::IracingConnection;
use crate::iracing::data_collector::IracingConnectionError;
use crate::iracing::data_collector::IracingValue;
use crate::iracing::data_collector::DataHeader;

fn extract_value<T>(telemetry: &[IracingValue], header: Option<(usize, &DataHeader)>, extractor: Box<dyn Fn(&IracingValue) -> T>) -> T {
    if let Some((idx, _)) = header {
        (extractor)(&telemetry[idx])
    } else {
        (extractor)(&IracingValue::Unknown)
    }
}

pub struct IracingTask {
    sender: Sender<Update>,
}

impl IracingTask {
    pub fn new(sender: Sender<Update>) -> IracingTask {
        IracingTask { sender }
    }

    pub async fn execute(self) {
        loop {
            let mut maybe_connection: Option<IracingConnection> = None;
            loop {
                match IracingConnection::new() {
                    Ok(new_connection) => {
                        maybe_connection = Some(new_connection);
                        break;
                    },
                    Err(IracingConnectionError::NotRunning) => {
                        info!("iRacing not detected. Retrying!");
                        std::thread::sleep(Duration::from_secs(1));
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
            let lap_dist_by_car_header = headers.iter().enumerate()
                    .find(|(_, header)| header.name == "CarIdxLapDistPct");
            let car_positions_header = headers.iter().enumerate()
                    .find(|(_, header)| header.name == "CarIdxPosition");
            let is_on_track_header = headers.iter().enumerate()
                    .find(|(_, header)| header.name == "IsOnTrack");

            let mut packages = 0;
            while let Some(package) = connection.next().await {
                packages += 1;
                if packages % 60 == 0 {
                    info!["Package count: {}", packages];
                }

                match package {
                    data_collector::Update::Telemetry(telemetry) => {
                        let throttle = extract_value(&telemetry, throttle_header, Box::new(|val| match val {
                            IracingValue::Float(throttle) => *throttle,
                            _ => 0.0
                        }));
                        let brake = extract_value(&telemetry, brake_header, Box::new(|val| match val {
                            IracingValue::Float(brake) => *brake,
                            _ => 0.0
                        }));
                        let lap_dist_by_car = extract_value(&telemetry, lap_dist_by_car_header, Box::new(|val| match val {
                            IracingValue::FloatVector(lap_dist_by_car) => lap_dist_by_car.clone(),
                            _ => vec![]
                        }));
                        let car_positions = extract_value(&telemetry, car_positions_header, Box::new(|val| match val {
                            IracingValue::IntVector(car_positions) => car_positions.clone(),
                            _ => vec![]
                        }));

                        let is_on_track = extract_value(&telemetry, is_on_track_header, Box::new(|val| match val {
                            IracingValue::Boolean(is_on_track) => *is_on_track,
                            _ => false,
                        }));

                        let timestamp = Instant::now();
                        self.sender.send(Update::Telemetry(Telemetry {
                            timestamp,
                            throttle,
                            brake,
                            gear: 1,
                            velocity: 0.0,
                            deltas: vec![],
                            lap_dist_by_car,
                            car_positions,
                            is_on_track,
                        })).await.unwrap();
                    },
                    data_collector::Update::SessionInfo(session_info_str) => {
                        info!["Session info: {}", session_info_str];

                        let session_info_sender = self.sender.clone();
                        task::spawn(async move {
                            match SessionInfo::try_from(&session_info_str) {
                                Ok(session_info) => session_info_sender.send(Update::Session(session_info)).await.unwrap(),
                                Err(err) => error!["Failed to parse session info: {}", err],
                            }
                        });
                    }
                }
            }

            // iRacing disconnected, wait 1 seconds before attempting to re-connect
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}

pub struct TestTask {
    sender: Sender<Update>,
}

impl TestTask {
    pub fn new(sender: Sender<Update>) -> TestTask {
        TestTask { sender }
    }

    pub async fn execute(self) {
        self.sender.send(Update::Session(SessionInfo {
            track: TrackSpec {
                name: "monza full".to_string(),
                configuration: "Grand Prix".to_string(),
            },
            driver: DriverInfo {
                car_idx: 1,
                username: "Test Driver".to_string(),
                irating: 1,
                license_string: "R 0.01".to_string(),
            }
        })).await.unwrap();

        let mut position = 0.0;
        let mut brake = 0.0;
        loop {
            position = (position + 0.001) % 1.0;
            brake = (brake + 0.05) % 1.05;
            self.sender.send(Update::Telemetry(Telemetry {
                timestamp: Instant::now(),
                throttle: 0.0,
                brake,
                gear: 1,
                velocity: 0.0,
                deltas: vec![0.364, 14.340, -2.423, -23.42],
                lap_dist_by_car: vec![0.0, position, 0.75],
                car_positions: vec![0, 1, 2],
                is_on_track: true,
            })).await.unwrap();

            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}
