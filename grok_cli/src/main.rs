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
        .arg(
            Arg::with_name("nocolor")
                .short("nc")
                .long("nocolor")
                .takes_value(false)
                .help("disable color highlighting"),
        )
        .get_matches();
    let nocolor = matches.is_present("nocolor");
    let level = matches.value_of("level").unwrap_or("ALL");
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.expect("Could not read line from standard in");
        let mut words: Vec<&str> = line.split_whitespace().collect();
        &words.remove(0);
        let instance = &words.remove(0);
        let json = &words.join(" ");
        let j = serde_json::from_str(&json);
        match j {
            Ok(l) => {
                let jm: JSONMessage = l;
                if level == "ALL" || level == jm.level {
                    let dt = Utc.timestamp((jm.timeMillis / 1000) as i64, 0);
                    println!(
                        "{}{} -- {} [{}] {}{} {}{} - {}{}{}",
                        if !nocolor {
                            color::Fg(color::Reset).to_string()
                        } else {
                            String::from("")
                        },
                        instance,
                        dt.to_rfc3339(),
                        jm.thread,
                        if !nocolor {
                            color::Fg(color::Magenta).to_string()
                        } else {
                            String::from("")
                        },
                        jm.level,
                        if !nocolor {
                            color::Fg(color::Reset).to_string()
                        } else {
                            String::from("")
                        },
                        jm.loggerName,
                        if !nocolor {
                            color::Fg(color::Cyan).to_string()
                        } else {
                            String::from("")
                        },
                        jm.message,
                        if !nocolor {
                            color::Fg(color::Reset).to_string()
                        } else {
                            String::from("")
                        }
                    );
                    match jm.thrown {
                        Some(t) => {
                            println!("{}", t.name);
                            for trace in t.extendedStackTrace {
                                println!(
                                    "\t at {}{}.{} ({}:{}) [{}]{}",
                                    if !nocolor {
                                        color::Fg(color::Red).to_string()
                                    } else {
                                        String::from("")
                                    },
                                    trace.class,
                                    trace.method,
                                    trace.file.unwrap_or("Unknown".to_string()),
                                    trace.line,
                                    trace.location,
                                    if !nocolor {
                                        color::Fg(color::Reset).to_string()
                                    } else {
                                        String::from("")
                                    },
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
                println!("Unable to parse line {}", e.to_string());
            }
        }
    }
}
