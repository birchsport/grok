extern crate clap;
extern crate grok_lib;
extern crate termion;
use termion::color;

use chrono::{TimeZone, Utc};
use clap::{App, Arg};
use grok_lib::log_json::module_json::*;
use std::io::{self, BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

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
            Arg::with_name("streams")
                .short("s")
                .long("streams")
                .takes_value(true)
                .help("CSV of all streams to read"),
        )
        .arg(
            Arg::with_name("nocolor")
                .short("nc")
                .long("nocolor")
                .takes_value(false)
                .help("disable color highlighting"),
        )
        .arg(
            Arg::with_name("raw")
                .short("r")
                .long("raw")
                .takes_value(false)
                .help("consume raw json (not from awslogs)"),
        )
        .get_matches();
    let nocolor = matches.is_present("nocolor");
    let raw = matches.is_present("raw");
    let level = matches.value_of("level").unwrap_or("ALL");
    if matches.is_present("streams") {
        let streams: Vec<&str> = matches.value_of("streams").unwrap().split(",").collect();
        for stream in streams {
            read_from_process(level.to_string(), nocolor, raw, stream.to_string());
        }
        loop {
            thread::sleep(Duration::from_millis(100));
        }
    } else {
        read_from_stdin(level.to_string(), nocolor, raw);
    }
}

fn read_from_stdin(level: String, nocolor: bool, raw: bool) {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(l) => {
                log_string(level.to_string(), nocolor, raw, l);
            }
            Err(e) => {
                println!("Unable to parse input {}", e.to_string());
            }
        }
    }
}

fn read_from_process(level: String, nocolor: bool, raw: bool, stream: String) {
    thread::spawn(move || {
        let mut cmd = Command::new("awslogs")
            .args(&["get", &stream, "--watch"])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let stdout = cmd.stdout.as_mut().unwrap();
            let stdout_reader = BufReader::new(stdout);
            let stdout_lines = stdout_reader.lines();

            for line in stdout_lines {
                match line {
                    Ok(l) => {
                        log_string(level.to_string(), nocolor, raw, l);
                    }
                    Err(e) => {
                        println!("Unable to parse input {}", e.to_string());
                    }
                }
            }
        }
        cmd.wait().unwrap();
    });
}

fn log_string(level: String, nocolor: bool, raw: bool, line: String) {
    let mut words: Vec<&str> = line.split_whitespace().collect();
    let mut stream = "";
    let mut instance = "";
    if ! raw {
        stream = &words.remove(0);
        instance = &words.remove(0);
    }
    let json = &words.join(" ");
    let j = serde_json::from_str(&json);
    match j {
        Ok(l) => {
            let jm: JSONMessage = l;
            if level == "ALL" || level == jm.level {
                let dt = Utc.timestamp((jm.timeMillis / 1000) as i64, 0);
                println!(
                    "{}{} {} -- {} [{}] {}{} {}{} - {}{}{}",
                    color_str(!nocolor, &color::Reset),
                    stream,
                    instance,
                    dt.to_rfc3339(),
                    jm.thread,
                    color_str(!nocolor, &color::Magenta),
                    jm.level,
                    color_str(!nocolor, &color::Reset),
                    jm.loggerName,
                    if jm.level == "ERROR" {
                        color_str(!nocolor, &color::Red)
                    } else if jm.level == "WARN" {
                        color_str(!nocolor, &color::Yellow)
                    } else {
                        color_str(!nocolor, &color::Cyan)
                    },
                    jm.message,
                    color_str(!nocolor, &color::Reset)
                );
                match jm.thrown {
                    Some(t) => {
                        println!("{}", t.name);
                        for trace in t.extendedStackTrace {
                            println!(
                                "\t at {}{}.{} ({}:{}) [{}]{}",
                                color_str(!nocolor, &color::Red),
                                trace.class,
                                trace.method,
                                trace.file.unwrap_or("Unknown".to_string()),
                                trace.line,
                                trace.location,
                                color_str(!nocolor, &color::Reset)
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
            println!("Unable to parse json {} :: input - {}", e.to_string(), line);
        }
    }
}

fn color_str(b: bool, c: &dyn color::Color) -> String {
    if b {
        return color::Fg(c).to_string();
    } else {
        return String::from("");
    }
}
