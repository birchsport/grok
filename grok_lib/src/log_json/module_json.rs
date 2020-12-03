use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
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
}

#[derive(Serialize, Deserialize)]
pub struct Thrown {
    pub commonElementCount: u32,
    pub name: String,
    pub extendedStackTrace: Vec<Trace>,
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
