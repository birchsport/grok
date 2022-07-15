use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct JSONMessage {
    pub timeMillis: Option<u64>,
    pub thread: String,
    pub level: String,
    pub loggerName: String,
    pub message: String,
    pub thrown: Option<Thrown>,
    pub threadId: i32,
    pub threadPriority: u32,
    pub endOfBatch: bool,
    pub loggerFqcn: String,
    pub instant: Option<Instant>,
    pub contextMap: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Thrown {
    pub commonElementCount: u32,
    pub name: String,
    pub message: Option<String>,
    pub extendedStackTrace: Vec<Trace>,
    pub cause: Option<Cause>
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Cause {
    pub commonElementCount: u32,
    pub name: String,
    pub message: String,
    pub extendedStackTrace: Vec<Trace>,
    pub cause: Option<SubCause>,
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SubCause {
    pub commonElementCount: u32,
    pub name: String,
    pub message: String,
    pub extendedStackTrace: Vec<Trace>,
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Instant {
    pub epochSecond: i64,
    pub nanoOfSecond: i64,
}

#[derive(Serialize, Deserialize)]
pub struct Trace {
    pub class: String,
    pub method: String,
    pub file: Option<String>,
    pub line: i32,
    pub exact: bool,
    pub location: String,
    pub version: String,
}
