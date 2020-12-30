extern crate clap;
extern crate termion;
use termion::color;

use grok::json::JSONMessage;

use chrono::{TimeZone, Utc};
use clap::{App, Arg};
use std::fmt::Write;
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
                println!("{}", create_log_string(level.to_string(), nocolor, raw, l));
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
                        println!("{}", create_log_string(level.to_string(), nocolor, raw, l));
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

fn create_log_string(level: String, nocolor: bool, raw: bool, line: String) -> String {
    let mut words: Vec<&str> = line.split_whitespace().collect();
    let mut stream = "";
    let mut instance = "";
    let mut line = String::new();
    if !raw {
        stream = &words.remove(0);
        instance = &words.remove(0);
    }
    let json = &words.join(" ");
    let j = serde_json::from_str(&json);
    match j {
        Ok(l) => {
            let jm: JSONMessage = l;
            if level == "ALL" || level == jm.level {
                let dt = Utc.timestamp((jm.instant.epochSecond) as i64, 0);
                writeln!(
                    line,
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
                        writeln!(line, "{}", t.name);
                        for trace in t.extendedStackTrace {
                            writeln!(
                                line,
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
    return line;
}

fn color_str(b: bool, c: &dyn color::Color) -> String {
    if b {
        return color::Fg(c).to_string();
    } else {
        return String::from("");
    }
}



#[cfg(test)]
mod tests {
    #[test]
    fn parse_line() {
        let input =  "prod-batch-ce-json prod-batch-1c004 {\"thread\":\"CommunicationEngineWorker-5\",\"level\":\"WARN\",\"loggerName\":\"com.shopstyle.messaging.ce.core.CommunicationRequestProcessor\",\"message\":\"Task type [CBReengageFavorite] took longer than [120] seconds to execute. Elapsed time: [3.471 min] - Request: [com.shopstyle.messaging.model.ce.CommunicationRequest@462d2036[id=7c60a640-b61c-4e55-812a-237568e93fd6,created=Mon Dec 21 11:31:22 CST 2020,source=5fe0dbc37be10c2ddad8cd46,appName=shopstyle,locale=en_US,types=[CBReengageFavorite],recipients=[40726490],frequencies={CBReengageFavorite=Monday},startDates=<null>,targets={CBReengageFavorite=[Email]},attributes=<null>,limit=1]]\",\"endOfBatch\":false,\"loggerFqcn\":\"org.apache.logging.slf4j.Log4jLogger\",\"instant\":{\"epochSecond\":1608579508,\"nanoOfSecond\":964000000},\"contextMap\":{},\"threadId\":95,\"threadPriority\":5}";

        let result = super::create_log_string("ALL".to_string(), false, false, input.to_string());
        assert_eq!(result.is_empty(), false);
    }
}
