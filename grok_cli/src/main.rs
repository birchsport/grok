extern crate clap;
extern crate grok_lib;
extern crate termion;
use termion::color;

use chrono::{TimeZone, Utc};
use clap::{App, Arg};
use grok_lib::log_json::module_json::*;
use std::io::{self, BufRead};

fn main() {
    let matches = App::new("grok")
        .version("0.1.0")
        .author("James Birchfield <jbirchfield@shopstyle.com>")
        .about("Reshapes JSON logging")
        .arg(
            Arg::with_name("level")
                .short("l")
                .possible_values(&["ALL", "TRACE", "DEBUG", "WARN", "INFO", "ERROR"])
                .default_value("ALL")
                .long("level")
                .takes_value(true)
                .help("filter to a certain log level"),
        )
        .get_matches();
    let level = matches.value_of("level").unwrap_or("ALL");
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.expect("Could not read line from standard in");
        let j = serde_json::from_str(&line);
        match j {
            Ok(l) => {
                let jm: JSONMessage = l;
                if level == "ALL" || level == jm.level {
                    let dt = Utc.timestamp((jm.timeMillis / 1000) as i64, 0);
                    println!(
                        "{}{} [{}] {}{} {}{} - {}{}{}",
                        color::Fg(color::White),
                        dt.to_rfc3339(),
                        jm.thread,
                        color::Fg(color::Magenta),
                        jm.level,
                        color::Fg(color::White),
                        jm.loggerName,
                        color::Fg(color::Cyan),
                        jm.message,
                        color::Fg(color::White)
                    );
                    match jm.thrown {
                        Some(t) => {
                            println!("{}", t.name);
                            for trace in t.extendedStackTrace {
                                println!(
                                    "\t at {}{}.{} ({}:{}) [{}]{}",
                                    color::Fg(color::Red),
                                    trace.class, 
                                    trace.method, 
                                    trace.file.unwrap_or("Unknown".to_string()),
                                    trace.line, 
                                    trace.location,
                                    color::Fg(color::White)
                                );
                            }
                        }
                        None => {
                            //swallow
                        }
                    }
                }
            }
            Err(e) => {
                // swallow
                println!("Unable to parse line {}", e.to_string());
            }
        }
    }
}
