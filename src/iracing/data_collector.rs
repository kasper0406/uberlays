#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::fs::File;
use std::ptr;
use std::mem;
use windows::{
    Win32::Foundation::*,
    Win32::System::Threading::*,
    Win32::System::Memory::*,
    Win32::Security::SECURITY_ATTRIBUTES,
};
use std::ffi::OsStr;
use std::ffi::CStr;
use std::os::windows::ffi::OsStrExt;

const SYNCHRONIZE: u32 = 0x00100000;
// const FILE_MAP_READ: u32 = 0x4;

struct PwString {
    content: Vec<u16>,
}

impl PwString {
    fn from(input: &str) -> PwString {
        let mut copy: Vec<u16> = OsStr::new(input).encode_wide().chain(Some(0)).collect();
        copy.push(0x0); // Null terminate string

        PwString { content: copy }
    }

    fn pwstr(&mut self) -> PWSTR {
        PWSTR(self.content.as_mut_ptr())
    }
}

#[derive(Debug)]
enum IracingValue {
    Double(f64),
    DoubleVector(Vec<f64>),
    Int(i32),
    IntVector(Vec<i32>),
    Float(f32),
    FloatVector(Vec<f32>),

    Unknown
}

pub struct IracingConnection {
    mem_file: HANDLE,
    header: *mut irsdk_header,
    event_file: HANDLE,
}

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
        let mut mmap_filename = PwString::from("Local\\IRSDKMemMapFileName");
        let mut event_filename = PwString::from("Local\\IRSDKDataValidEvent");

        let mem_file = unsafe { OpenFileMappingW(FILE_MAP_READ.0, false, mmap_filename.pwstr()) };
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
        
        let event_file = unsafe { OpenEventW(SYNCHRONIZE, false, event_filename.pwstr()) };
        drop(event_filename);
        info!("Event handle: {:?}, error: {:?}", event_file, unsafe { GetLastError() });

        Ok(IracingConnection {
            mem_file, header, event_file
        })

        /*
        let mut buffer = vec![];
        for i in 0..1 {
            match unsafe { WaitForSingleObject(event_file, u32::MAX) } {
                0x0 => (),
                0x102 => panic!("TIMEOUT!"),
                err => panic!("Some other failure: {}, detailed: {:?}", err, unsafe { GetLastError() })
            };

            info!("Buffer header: {}", unsafe { (*header).ver });
            info!("Status: {}", unsafe { (*header).status });
            info!("Tickrate: {}", unsafe { (*header).tickRate });
            info!("Session info update: {}", unsafe { (*header).sessionInfoUpdate });
            info!("numVars: {}", unsafe { (*header).numVars });
            info!("varHeader offset: {}", unsafe { (*header).varHeaderOffset });
            info!("tickCount[0]: {}", unsafe { (*header).varBuf[0].tickCount });
            info!("tickCount[1]: {}", unsafe { (*header).varBuf[1].tickCount });
            info!("tickCount[2]: {}", unsafe { (*header).varBuf[2].tickCount });
            info!("");

            let new_buffer_length = unsafe { (*header).bufLen } as usize;
            if buffer.len() != new_buffer_length {
                buffer = vec![0u8; new_buffer_length];
            }

            unsafe {
                for i in 0..(unsafe { (*header).numVars } as isize) {
                    let var_headers = (mmap as *const u8).offset((*header).varHeaderOffset as isize) as *mut irsdk_varHeader;
                    let var_header = *((var_headers.offset(i)) as *const irsdk_varHeader) as irsdk_varHeader;
        
                    let name = CStr::from_ptr(var_header.name.as_ptr());
                    let desc = CStr::from_ptr(var_header.desc.as_ptr());
                    let unit = CStr::from_ptr(var_header.unit.as_ptr());
        
                    // info!("Offset: {}", var_header.offset);
                    // info!("Count: {}", var_header.count);

                    let idx = (*header).varBuf.iter().enumerate().max_by_key(|(_, buf)| buf.tickCount).unwrap().0;
                    let tick_count_before = (*header).varBuf[idx].tickCount;

                    let values_ptr = (mmap as *const u8).offset((*header).varBuf[idx].bufOffset as isize);
                    buffer.copy_from_slice(std::slice::from_raw_parts(values_ptr, new_buffer_length));

                    if (*header).varBuf[idx].tickCount != tick_count_before {
                        panic!("Data changed while copying! This can't be good!")
                    }

                    let buffer_ptr = buffer.as_ptr();
        
                    let value_ptr = buffer_ptr.offset(var_header.offset as isize);
                    let value = match var_header.type_ {
                        irsdk_VarType_irsdk_double => IracingValue::Double((*(value_ptr as *const f64)).clone()),
                        irsdk_VarType_irsdk_int => IracingValue::Int((*(value_ptr as *const i32)).clone()),
                        irsdk_VarType_irsdk_float => IracingValue::Float((*(value_ptr as *const f32)).clone()),
                        _ => IracingValue::Unknown
                    };
        
                    // if name.to_str().unwrap() == "Throttle" {
                    info!("{} [{}]: {}", name.to_str().unwrap(), unit.to_str().unwrap(), desc.to_str().unwrap());
                    info!("    - {:?}", value);
                    // }
                }
            }
        } */
    }
}
