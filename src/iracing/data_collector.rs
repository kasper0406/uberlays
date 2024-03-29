#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));


use async_std::sync::Mutex;

use windows::core::PCWSTR;
use windows::{
    Win32::Foundation::*,
    Win32::System::Threading::*,
    Win32::System::Memory::*,
};
use std::ffi::OsStr;
use std::ffi::CStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::Arc;
use std::time::Duration;
use async_std::task;
use async_std::task::{Context, Poll};
use async_std::stream::Stream;
use async_std::pin::Pin;

const SYNCHRONIZE: u32 = 0x00100000;
// const FILE_MAP_READ: u32 = 0x4;

struct PcwString {
    content: Vec<u16>,
}

impl PcwString {
    fn from(input: &str) -> PcwString {
        let mut copy: Vec<u16> = OsStr::new(input).encode_wide().chain(Some(0)).collect();
        copy.push(0x0); // Null terminate string

        PcwString { content: copy }
    }

    fn pcwstr(&mut self) -> PCWSTR {
        PCWSTR(self.content.as_mut_ptr())
    }
}

#[derive(Debug)]
pub enum IracingValue {
    Double(f64),
    DoubleVector(Vec<f64>),
    Int(i32),
    IntVector(Vec<i32>),
    Float(f32),
    FloatVector(Vec<f32>),
    Boolean(bool),

    Unknown
}

#[derive(Debug)]
pub struct DataHeader {
    pub name: String,
    pub description: String,
    pub unit: String,
}

pub struct IracingConnection {
    mem_file: HANDLE,
    header: *mut irsdk_header,
    event_file: HANDLE,

    seen_tick_count: i32,
    buffer: Vec<u8>,

    session_info_seen_tick_count: i32,

    should_disconnect: Arc<Mutex<bool>>,
}

unsafe impl Send for IracingConnection {}

impl Drop for IracingConnection {
    fn drop(self: &mut IracingConnection) {
        info!("Dropping iRacing connection!");

        unsafe {
            CloseHandle(self.event_file);
            UnmapViewOfFile(self.header as *mut std::ffi::c_void);
            CloseHandle(self.mem_file);
        }
    }
}

pub enum IracingConnectionError {
    NotRunning,
}

impl IracingConnection {
    pub fn new() -> Result<IracingConnection, IracingConnectionError> {
        let mut mmap_filename = PcwString::from("Local\\IRSDKMemMapFileName");
        let mut event_filename = PcwString::from("Local\\IRSDKDataValidEvent");

        let mem_file = unsafe { OpenFileMappingW(FILE_MAP_READ.0, false, mmap_filename.pcwstr()) }.unwrap();
        drop(mmap_filename);

        info!("Mmap file: {:?}, error: {:?}", mem_file, unsafe { GetLastError() });

        let mmap = unsafe { MapViewOfFile(mem_file, FILE_MAP_READ, 0, 0, 0) };
        info!("Mmap handle: {:?}, error: {:?}", mmap, unsafe { GetLastError() });
        if mmap.is_null() {
            info!("iRacing session is not running");
            return Err(IracingConnectionError::NotRunning);
        }
        info!("The memmap is set up!");

        let header = mmap as *mut irsdk_header;

        let event_file = unsafe { OpenEventW(SYNCHRONIZE, false, event_filename.pcwstr()) }.unwrap();
        drop(event_filename);
        info!("Event handle: {:?}, error: {:?}", event_file, unsafe { GetLastError() });

        Ok(IracingConnection {
            mem_file, header, event_file,
            seen_tick_count: -1,
            session_info_seen_tick_count: -1,
            buffer: vec![],
            should_disconnect: Arc::new(Mutex::new(false)),
        })
    }

    pub fn headers(&self) -> Vec<DataHeader> {
        unsafe {
            let num_headers = (*self.header).numVars as isize;
            let mut headers = Vec::with_capacity(num_headers as usize);

            let var_headers = (self.header as *const u8).offset((*self.header).varHeaderOffset as isize) as *mut irsdk_varHeader;

            for i in 0..num_headers {
                let var_header = *((var_headers.offset(i)) as *const irsdk_varHeader) as irsdk_varHeader;

                let name = String::from(CStr::from_ptr(var_header.name.as_ptr()).to_str().unwrap());
                let description = String::from(CStr::from_ptr(var_header.desc.as_ptr()).to_str().unwrap());
                let unit = String::from(CStr::from_ptr(var_header.unit.as_ptr()).to_str().unwrap());

                headers.push(DataHeader { name, description, unit });
            }

            headers
        }
    }
}

fn latin1_to_string(buffer: &[u8]) -> String {
    buffer.iter().map(|&c| c as char).collect()
}

pub enum Update {
    Telemetry(Vec<IracingValue>),
    SessionInfo(String),
}

const EVENT_TIMEOUT: Duration = Duration::from_secs(2);

impl Stream for IracingConnection {
    type Item = Update;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(should_disconnect) =  self.should_disconnect.try_lock() {
            if *should_disconnect {
                return Poll::Ready(None);
            }
        }

        unsafe {
            // TODO(knielsen): We could be timed here, but for now :shrug:
            let idx = (*self.header).varBuf.iter().enumerate().max_by_key(|(_, buf)| buf.tickCount).unwrap().0;
            let tick_count_before = (*self.header).varBuf[idx].tickCount;
            let session_info_tick_count = (*self.header).sessionInfoUpdate;

            // Check for session info updates
            if session_info_tick_count > self.session_info_seen_tick_count {
                self.session_info_seen_tick_count = session_info_tick_count;
                debug!["Updating session info at tick {}", self.session_info_seen_tick_count];

                let len = (*self.header).sessionInfoLen as usize;
                if len == 0 {
                    // If the Session Info provided is empty, it seems like iRacing will continue to keep
                    // the session open, despite writing the actual data to another session. Instead we will
                    // close the connection, and make the code open a new one, to connect to the new session
                    return Poll::Ready(None);
                }

                let offset = (*self.header).sessionInfoOffset as isize;
                let mut buffer = vec![0u8; len];

                let session_info_ptr = (self.header as *const u8).offset(offset);
                buffer.copy_from_slice(std::slice::from_raw_parts(session_info_ptr, len));
                if (*self.header).sessionInfoUpdate != session_info_tick_count {
                    panic!("Session info was changed while copying!");
                }

                return Poll::Ready(Some(Update::SessionInfo(latin1_to_string(&buffer))))
            }

            // Check for ordinary Telemetry updates
            if tick_count_before <= self.seen_tick_count {
                // Wait for a new element
                let waiter = cx.waker().clone();
                let disconnect_setter = self.should_disconnect.clone();
                let event_file_clone = self.event_file;
                task::spawn(async move {
                    match WaitForSingleObject(event_file_clone, EVENT_TIMEOUT.as_millis().try_into().unwrap()) {
                        0x0 => waiter.wake(),
                        0x102 => {
                            info!["Disconnected from iRacing after event timeout"];
                            *disconnect_setter.lock().await = true;
                            waiter.wake()
                        },
                        err => panic!("Some other failure: {}, detailed: {:?}", err, GetLastError())
                    };
                });

                Poll::Pending
            } else {
                self.seen_tick_count = tick_count_before;

                let new_buffer_length = (*self.header).bufLen as usize;
                if self.buffer.len() != new_buffer_length {
                    self.buffer = vec![0u8; new_buffer_length];
                }

                let values_ptr = (self.header as *const u8).offset((*self.header).varBuf[idx].bufOffset as isize);
                self.buffer.copy_from_slice(std::slice::from_raw_parts(values_ptr, new_buffer_length));
                if (*self.header).varBuf[idx].tickCount != tick_count_before {
                    panic!("Data changed while copying! This can't be good!");
                }

                let num_headers = (*self.header).numVars as isize;
                let mut values = Vec::with_capacity(num_headers as usize);
                let var_headers = (self.header as *const u8).offset((*self.header).varHeaderOffset as isize) as *mut irsdk_varHeader;
                for i in 0..num_headers {
                    let var_header = *((var_headers.offset(i)) as *const irsdk_varHeader) as irsdk_varHeader;

                    let value_ptr = self.buffer.as_ptr().offset(var_header.offset as isize);
                    let count = var_header.count as usize;
                    let value = match (var_header.type_, count) {
                        (irsdk_VarType_irsdk_double, 1) => IracingValue::Double(*(value_ptr as *const f64)),
                        (irsdk_VarType_irsdk_double, _) => {
                            let mut values = Vec::with_capacity(count);
                            for j in 0..count {
                                values.push(*(value_ptr as *const f64).add(j));
                            }
                            IracingValue::DoubleVector(values)
                        },
                        (irsdk_VarType_irsdk_int, 1) => IracingValue::Int(*(value_ptr as *const i32)),
                        (irsdk_VarType_irsdk_int, _) => {
                            let mut values = Vec::with_capacity(count);
                            for j in 0..count {
                                values.push(*(value_ptr as *const i32).add(j));
                            }
                            IracingValue::IntVector(values)
                        },
                        (irsdk_VarType_irsdk_float, 1) => IracingValue::Float(*(value_ptr as *const f32)),
                        (irsdk_VarType_irsdk_float, _) => {
                            let mut values = Vec::with_capacity(count);
                            for j in 0..count {
                                values.push(*(value_ptr as *const f32).add(j));
                            }
                            IracingValue::FloatVector(values)
                        },
                        (irsdk_VarType_irsdk_bool, 1) => IracingValue::Boolean(*(value_ptr as *const u8) != 0),
                        _ => IracingValue::Unknown
                    };

                    values.push(value);
                }

                Poll::Ready(Some(Update::Telemetry(values)))
            }
        }
    }
}
