extern crate chrono;
extern crate chrono_english;
extern crate clap;
extern crate termion;

use std::collections::HashMap;
use std::fmt::Write;
use std::io::{self, BufRead};
use std::ops::Deref;
use std::str::FromStr;
use std::thread;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono::prelude::*;
use chrono_english::{Dialect, parse_date_string};
use clap::{App, Arg, crate_version};
use log::{debug, error, warn};
use rusoto_core::Region;
use rusoto_logs::{CloudWatchLogs, CloudWatchLogsClient, DescribeLogGroupsRequest, FilterLogEventsRequest};
use termion::color;

use grok::json::JSONMessage;

#[derive(Clone)]
struct Config {
    region: String,
    nocolor: bool,
    level: String,
    group: String,
    start_date: Option<String>,
    end_date: Option<String>,
    pattern: Option<String>,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let matches = App::new("grok")
        .version(crate_version!())
        .author("James Birchfield <jbirchfield@demeterlogistics.com>")
        .about("Streams Cloudwatch Logs")
        .arg(
            Arg::with_name("region")
                .short("r")
                .long("region")
                .takes_value(true)
                .default_value("us-east-1")
                .help("optional region"),
        )
        .arg(
            Arg::with_name("start")
                .short("s")
                .long("start")
                .takes_value(true)
                .help("optional start date (i.e. 1 hour ago)"),
        )
        .arg(
            Arg::with_name("end")
                .short("e")
                .long("end")
                .takes_value(true)
                .help("optional end date (i.e. now, 1 hour ago)"),
        )
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
            Arg::with_name("pattern")
                .short("p")
                .long("pattern")
                .takes_value(true)
                .help("Optional pattern to match"),
        )
        .arg(
            Arg::with_name("groups")
                .short("g")
                .long("groups")
                .takes_value(true)
                .help("CSV of all groups to read"),
        )
        .arg(
            Arg::with_name("list")
                .long("list")
                .takes_value(false)
                .help("lists log groups only"),
        )
        .arg(
            Arg::with_name("nocolor")
                .short("nc")
                .long("nocolor")
                .takes_value(false)
                .help("disable color highlighting"),
        )
        .get_matches();

    let region = matches.value_of("region").unwrap_or("us-east-1");
    let lg = matches.is_present("list");
    if lg {
        let _list_groups = list_groups(region);
        _list_groups.await;
    } else {
        let nocolor = matches.is_present("nocolor");
        let mut start_date = None;
        let mut end_date = None;
        let mut pattern = None;
        if matches.is_present("start") {
            start_date = Some(String::from(matches.value_of("start").unwrap()));
        }
        if matches.is_present("end") {
            end_date = Some(String::from(matches.value_of("end").unwrap()));
        }
        if matches.is_present("pattern") {
            pattern = Some(String::from(matches.value_of("pattern").unwrap()));
        }
        let level = matches.value_of("level").unwrap_or("ALL");
        if matches.is_present("groups") {
            let mut handles = vec![];
            let mut groups: Vec<String> = vec![];
            let group_str = matches.value_of("groups").unwrap();
            if group_str.starts_with("all") {
                let group_opts: Vec<&str> = group_str.split(":").collect();
                let filter_csv = group_opts.get(1).unwrap().to_string();
                let filter_opts: Vec<&str> = filter_csv.split(",").collect();
                let mut all_groups: Vec<String> = get_groups(region).await
                    .into_iter()
                    .filter(|group|
                        filter_opts.iter().any(|&f|
                            group.to_string().contains(&f.to_string())))
                    .collect();
                groups.append(&mut all_groups);
            } else {
                let provided_groups: Vec<&str> = matches.value_of("groups").unwrap().split(",").collect();
                let mut pgs: Vec<String> = provided_groups.iter().map(|&s| s.into()).collect();
                groups.append(&mut pgs);
            }
            if groups.len() > 10 {
                println!("Only showing first 8 groups");
            }
            for x in (0..8) {
                let group_o = groups.get(x);
                if group_o.is_none() {
                    break;
                }
                let group = group_o.unwrap();
                let config = Config {
                    region: region.to_string(),
                    nocolor: nocolor,
                    level: level.to_string(),
                    group: group.to_string(),
                    start_date: start_date.clone(),
                    end_date: end_date.clone(),
                    pattern: pattern.clone(),
                };
                let jh = tokio::spawn(async move {
                    println!("Reading from group {}", config.group);
                    read_from_cloudwatch(config).await;
                });
                handles.push(jh);
            }
            futures::future::join_all(handles).await;
        } else {
            read_from_stdin(level.to_string(), nocolor);
        }
    }
}

async fn read_from_cloudwatch(config: Config) {
    let mut end;
    let mut start;
    let mut watch = true;
    if config.end_date.is_some() {
        let end_date =
            parse_date_string(&*config.end_date.unwrap(), Local::now(), Dialect::Us);
        match end_date {
            Ok(v) => {
                end = v.timestamp_millis();
            }
            Err(_e) => {
                end = Utc::now().timestamp_millis();
            }
        }
        watch = false;
    } else {
        end = Utc::now().timestamp_millis();
    }
    if config.start_date.is_some() {
        let start_date =
            parse_date_string(&*config.start_date.unwrap(), Local::now(), Dialect::Us);
        match start_date {
            Ok(v) => {
                start = v.timestamp_millis();
            }
            Err(_e) => {
                start = end - 120000;
            }
        }
        watch = false;
    } else {
        start = end - 120000;
    }
    'outer: loop {
        let mut get_log_req: FilterLogEventsRequest = Default::default();
        get_log_req.log_group_name = config.group.clone();
        // we have to account for the ~10s it takes to ingest the logs, so we always look back 10 seconds
        get_log_req.start_time = Some(start - 10000);
        get_log_req.end_time = Some(end - 10000);
        if config.pattern.is_some() {
            get_log_req.filter_pattern = config.pattern.clone();
        }
        debug!("Start: {}", get_log_req.start_time.unwrap());
        debug!("End: {}", get_log_req.end_time.unwrap());
        debug!("Group: {}", get_log_req.log_group_name);
        debug!("Range: {}", end - start);

        let client = CloudWatchLogsClient::new(Region::from_str(&*config.region).unwrap());
        let mut next_token = None;
        'inner: loop {
            get_log_req.next_token = next_token;
            let get_log_resp = client
                .filter_log_events(get_log_req.clone())
                .await
                .unwrap_or_else(|e| panic!("Failed on get log events: {}", e));

            debug!("Found {} events", get_log_resp.events.clone().unwrap().len());
            for event in get_log_resp.events {
                for outp in event {
                    let msg = outp.message.unwrap();
                    debug!("{}", msg.clone().to_string());
                    let stream = outp.log_stream_name.unwrap();
                    let line = create_log_string(
                        config.level.clone(),
                        config.group.clone(),
                        stream,
                        config.nocolor,
                        msg.to_string(),
                    );
                    if line.len() > 0 {
                        println!("{}", line);
                    }
                }
            }
            next_token = get_log_resp.next_token;
            if next_token.is_none() {
                break 'inner;
            }
            thread::sleep(std::time::Duration::from_millis(100));
        }
        if !watch {
            break 'outer;
        }
        thread::sleep(std::time::Duration::from_millis(2000));
        start = end.clone();
        end = Utc::now().timestamp_millis();
    };
}

async fn list_groups(region: &str) {
    let client = CloudWatchLogsClient::new(Region::from_str(&*region).unwrap());
    let mut next_token = None;
    loop {
        let mut desc_groups_req: DescribeLogGroupsRequest = Default::default();
        desc_groups_req.next_token = next_token;
        let desc_groups_resp = client
            .describe_log_groups(desc_groups_req)
            .await
            .unwrap_or_else(|e| panic!("Failed on get log groups: {}", e));

        next_token = desc_groups_resp.next_token;
        for event in desc_groups_resp.log_groups {
            for lg in event {
                let msg = lg.log_group_name.unwrap();
                println!("{}", msg);
            }
        }
        if next_token.is_none() {
            break;
        }
        thread::sleep(std::time::Duration::from_millis(100));
    };
}

async fn get_groups(region: &str) -> Vec<String> {
    let client = CloudWatchLogsClient::new(Region::from_str(&*region).unwrap());
    let mut next_token = None;
    let mut groups = vec![];

    loop {
        let mut desc_groups_req: DescribeLogGroupsRequest = Default::default();
        desc_groups_req.next_token = next_token;
        let desc_groups_resp = client
            .describe_log_groups(desc_groups_req)
            .await
            .unwrap_or_else(|e| panic!("Failed on get log groups: {}", e));

        next_token = desc_groups_resp.next_token;
        for event in desc_groups_resp.log_groups {
            for lg in event {
                let msg = lg.log_group_name.unwrap();
                groups.push(msg.clone());
            }
        }
        if next_token.is_none() {
            break;
        }
        thread::sleep(std::time::Duration::from_millis(100));
    };
    return groups;
}

fn read_from_stdin(level: String, nocolor: bool) {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(l) => {
                let line = create_log_string(level.to_string(), String::new(), String::new(),
                                             nocolor, l);
                if line.len() > 0 {
                    println!("{}", line);
                }
            }
            Err(e) => {
                error!("Unable to parse input {}", e.to_string());
            }
        }
    }
}

fn create_log_string(
    level: String,
    group: String,
    stream: String,
    nocolor: bool,
    line: String,
) -> String {
    debug!("Line: {}", line.clone());
    debug!("Group: {}", group.clone());
    debug!("Stream: {}", stream.clone());
    debug!("Level: {}", level.clone());
    let mut out_line = String::new();
    let j = serde_json::from_str(&line);
    match j {
        Ok(l) => {
            let jm: JSONMessage = l;
            if level == "ALL" || level == jm.level {
                let dt = NaiveDateTime::from_timestamp(jm.instant.unwrap()
                                                           .epochSecond, 0);
                // let dt = Utc.timestamp((jm.timeMillis.unwrap()) as i64, 0);
                write!(
                    out_line,
                    "{}{} {} -- {} [{}] {}{} {}{} - {}{}{}",
                    color_str(!nocolor, &color::Reset),
                    group,
                    stream,
                    dt,
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

                match jm.contextMap {
                    Some(m) => {
                        if !m.is_empty() {
                            writeln!(out_line, "Context map: ");
                            for (k, v) in m {
                                writeln!(
                                    out_line,
                                    "\t {} = {}", k, v);
                            }
                        }
                    }
                    None => {
                        // swallow
                    }
                }
                //TODO don't really like this level of nesting, but leaving it for now
                match jm.thrown {
                    Some(t) => {
                        writeln!(out_line, "Stacktrace: {} - {}", t.name, t.message.unwrap_or("none".to_string()));
                        for trace in t.extendedStackTrace {
                            writeln!(
                                out_line,
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
                        match t.cause {
                            Some(c) => {
                                writeln!(out_line, "Caused by: {} - {}", c.name, c.message);
                                for ctrace in c.extendedStackTrace {
                                    writeln!(
                                        out_line,
                                        "\t at {}{}.{} ({}:{}) [{}]{}",
                                        color_str(!nocolor, &color::Red),
                                        ctrace.class,
                                        ctrace.method,
                                        ctrace.file.unwrap_or("Unknown".to_string()),
                                        ctrace.line,
                                        ctrace.location,
                                        color_str(!nocolor, &color::Reset)
                                    );
                                }
                                match c.cause {
                                    Some(sc) => {
                                        writeln!(out_line, "Caused by: {} - {}", sc.name, sc.message);
                                        for sctrace in sc.extendedStackTrace {
                                            writeln!(
                                                out_line,
                                                "\t at {}{}.{} ({}:{}) [{}]{}",
                                                color_str(!nocolor, &color::Red),
                                                sctrace.class,
                                                sctrace.method,
                                                sctrace.file.unwrap_or("Unknown".to_string()),
                                                sctrace.line,
                                                sctrace.location,
                                                color_str(!nocolor, &color::Reset)
                                            );
                                        }
                                    }
                                    None => {
                                        //swallow
                                    }
                                }
                            }
                            None => {
                                //swallow
                            }
                        }
                    }
                    None => {
                        //swallow
                    }
                }
            }
        }
        Err(_e) => {
            warn!("Exception: {}", _e);
            write!(out_line, "{} {} -- {}", group.clone(), stream.clone(), line);
        }
    }
    return out_line;
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
        let input = "{\"thread\":\"CommunicationEngineWorker-5\",\"level\":\"WARN\",\"loggerName\":\"com.shopstyle.messaging.ce.core.CommunicationRequestProcessor\",\"message\":\"Task type [CBReengageFavorite] took longer than [120] seconds to execute. Elapsed time: [3.471 min] - Request: [com.shopstyle.messaging.model.ce.CommunicationRequest@462d2036[id=7c60a640-b61c-4e55-812a-237568e93fd6,created=Mon Dec 21 11:31:22 CST 2020,source=5fe0dbc37be10c2ddad8cd46,appName=shopstyle,locale=en_US,types=[CBReengageFavorite],recipients=[40726490],frequencies={CBReengageFavorite=Monday},startDates=<null>,targets={CBReengageFavorite=[Email]},attributes=<null>,limit=1]]\",\"endOfBatch\":false,\"loggerFqcn\":\"org.apache.logging.slf4j.Log4jLogger\",\"instant\":{\"epochSecond\":1608579508,\"nanoOfSecond\":964000000},\"contextMap\":{},\"threadId\":95,\"threadPriority\":5}";

        let result = super::create_log_string(
            "ALL".to_string(),
            String::new(),
            String::new(),
            true,
            input.to_string(),
        );
        assert_eq!(result.is_empty(), false);
    }
}
