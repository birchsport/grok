extern crate grok_lib;

use std::io::{self, BufRead};
use grok_lib::log_json::module_json::*;
use chrono::{DateTime, TimeZone, Utc};


fn main() {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.expect("Could not read line from standard in");
        let jm: JSONMessage = serde_json::from_str(&line).unwrap();
        let dt = Utc.timestamp((jm.timeMillis/ 1000) as i64, 0);
        println!(
            "{} [{}] {} {} - {}",
            dt.to_rfc3339(),
            jm.thread,
            jm.level,
            jm.loggerName,
            jm.message
        );
        match jm.thrown {
            Some(t) => {
                println!("{}", t.name);
                for trace in t.extendedStackTrace {
                    let file = show_file(trace.file);
                    println!(
                        "\t at {}.{} ({}:{}) [{}]",
                        trace.class, trace.method, file, trace.line, trace.location
                    );
                }
            }
            None => {}
        }
    }

    fn show_file(name: Option<String>) -> String {
        return name.unwrap_or("Unknown".to_string());
    }
}
