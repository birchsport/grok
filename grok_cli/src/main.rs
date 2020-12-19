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
use std::fmt::Write;

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
                let dtv = match jm.timeMillis {
                    Some(val) => val,
                    None => 0
                };
                let dt = Utc.timestamp((dtv / 1000) as i64, 0);
                writeln!(line, "{}{} {} -- {} [{}] {}{} {}{} - {}{}{}",
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
                            writeln!(line, "\t at {}{}.{} ({}:{}) [{}]{}",
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

//TODO How do I test this???
// #[cfg(test)]
// mod tests {
//     #[test]
//     fn parse_line() {

//         let foo = "prod-api-store-json prod-api-1c003 { \"timeMillis\": 1586449144853, \"thread\": \"catalina-exec-44\", \"level\": \"ERROR\", \"loggerName\": \"com.shopstyle.app.api.APIService.serverErrors\", \"message\": \"\\nMethod: GET\\nURL: https://www.shopstyleqa.com:443/api/v2/favorites?authDebug=1&includeStackTrace=1&limit=100&objectType=Brand&offset=0&pid=shopstyle\\nIdentifier: 43370716:qa-gardian-1586449117QSG\\nTheme: shopstyle\\nLocale: en_US\\nParameters: authDebug = 1, includeStackTrace = 1, limit = 100, objectType = Brand, offset = 0, pid = shopstyle\\nHeaders: host = www.shopstyleqa.com, x-forwarded-for = 34.226.120.102, 34.195.252.201, 172.18.13.123, x-forwarded-proto = https, x-forwarded-port = 443, x-amzn-trace-id = Root=1-5e8f4af8-4f4d9f9a8077677481e45974, authorization = PopSugar userId=43370716, loginTimestamp=1586449141, digest=iTf6pNv3zhtlJfvwt+SOcgza3To=, version=0, x-amz-cf-id = Uj08peDQU_Y2yyuGd1wNw9KVibwB-XAJOkdyC8kp8F1joW8bX-S1PQ==, cookie = attribution=%7B%22medium%22%3A%22direct%22%7D; userData=%7B%22id%22%3A43370716%2C%22handle%22%3A%22qa-gardian-1586449117QSG%22%2C%22authLevel%22%3A%22Registered%22%2C%22loginToken%22%3A%22941a2f14cdaad879cc435c90f67639096e5c65567965e32a33ad498ce733db22%22%2C%22loginTimestamp%22%3A%221586449141%22%2C%22newUser%22%3Afalse%2C%22type%22%3A%22Standard%22%2C%22rewardsStatus%22%3A%22Unenrolled%22%7D, accept = application/json, referer = https://www.shopstyleqa.com/?abtest=collectiveCashBack:false,elasticSearch:true,expandedSearch:false,feedCompletionMeter:false,landingPrompt:false,materialBoosting:false,megaMenu:click,newness:false,pwa:false,segmentedSearch:false,signupBonus:false,signupSplash:false,smartFeed:false&trackDebug=true, user-agent = Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/66.0.3359.117 Safari/537.36 Gardian/0.2076, via = 2.0 09e7a54b3c0e42cf23f1deb97f4f6b95.cloudfront.net (CloudFront), cloudfront-viewer-country = US, accept-encoding = gzip, x-ss-seotts = trafficStage=false, x-ss-sid = 5e8f4ad2065046bd1cde116b, authorization-date = Thu, 09 Apr 2020 16:19:04 GMT, content-type = application/json, sec-fetch-site = same-origin, sec-fetch-mode = cors, x-forwarded-host = www.shopstyleqa.com, x-forwarded-server = shopstyle.com, connection = Keep-Alive\\nIP: 34.226.120.102\\n\\n\", \"thrown\": { \"commonElementCount\": 0, \"name\": \"java.lang.NullPointerException\", \"extendedStackTrace\": [ { \"class\": \"com.shopstyle.store.api.common.resource.FavoriteAPI\", \"method\": \"getFavorites\", \"file\": \"FavoriteAPI.java\", \"line\": 590, \"exact\": false, \"location\": \"classes/\", \"version\": \"?\" }, { \"class\": \"jdk.internal.reflect.GeneratedMethodAccessor1298\", \"method\": \"invoke\", \"line\": -1, \"exact\": false, \"location\": \"?\", \"version\": \"?\" }, { \"class\": \"jdk.internal.reflect.DelegatingMethodAccessorImpl\", \"method\": \"invoke\", \"file\": \"DelegatingMethodAccessorImpl.java\", \"line\": 43, \"exact\": false, \"location\": \"?\", \"version\": \"?\" }, { \"class\": \"java.lang.reflect.Method\", \"method\": \"invoke\", \"file\": \"Method.java\", \"line\": 566, \"exact\": false, \"location\": \"?\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.server.model.internal.ResourceMethodInvocationHandlerFactory$1\", \"method\": \"invoke\", \"file\": \"ResourceMethodInvocationHandlerFactory.java\", \"line\": 81, \"exact\": false, \"location\": \"jersey-server-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.server.model.internal.AbstractJavaResourceMethodDispatcher$1\", \"method\": \"run\", \"file\": \"AbstractJavaResourceMethodDispatcher.java\", \"line\": 144, \"exact\": false, \"location\": \"jersey-server-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.server.model.internal.AbstractJavaResourceMethodDispatcher\", \"method\": \"invoke\", \"file\": \"AbstractJavaResourceMethodDispatcher.java\", \"line\": 161, \"exact\": false, \"location\": \"jersey-server-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.server.model.internal.JavaResourceMethodDispatcherProvider$TypeOutInvoker\", \"method\": \"doDispatch\", \"file\": \"JavaResourceMethodDispatcherProvider.java\", \"line\": 205, \"exact\": false, \"location\": \"jersey-server-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.server.model.internal.AbstractJavaResourceMethodDispatcher\", \"method\": \"dispatch\", \"file\": \"AbstractJavaResourceMethodDispatcher.java\", \"line\": 99, \"exact\": false, \"location\": \"jersey-server-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.server.model.ResourceMethodInvoker\", \"method\": \"invoke\", \"file\": \"ResourceMethodInvoker.java\", \"line\": 389, \"exact\": false, \"location\": \"jersey-server-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.server.model.ResourceMethodInvoker\", \"method\": \"apply\", \"file\": \"ResourceMethodInvoker.java\", \"line\": 347, \"exact\": false, \"location\": \"jersey-server-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.server.model.ResourceMethodInvoker\", \"method\": \"apply\", \"file\": \"ResourceMethodInvoker.java\", \"line\": 102, \"exact\": false, \"location\": \"jersey-server-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.server.ServerRuntime$2\", \"method\": \"run\", \"file\": \"ServerRuntime.java\", \"line\": 326, \"exact\": true, \"location\": \"jersey-server-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.internal.Errors$1\", \"method\": \"call\", \"file\": \"Errors.java\", \"line\": 271, \"exact\": true, \"location\": \"jersey-common-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.internal.Errors$1\", \"method\": \"call\", \"file\": \"Errors.java\", \"line\": 267, \"exact\": true, \"location\": \"jersey-common-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.internal.Errors\", \"method\": \"process\", \"file\": \"Errors.java\", \"line\": 315, \"exact\": true, \"location\": \"jersey-common-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.internal.Errors\", \"method\": \"process\", \"file\": \"Errors.java\", \"line\": 297, \"exact\": true, \"location\": \"jersey-common-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.internal.Errors\", \"method\": \"process\", \"file\": \"Errors.java\", \"line\": 267, \"exact\": true, \"location\": \"jersey-common-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.process.internal.RequestScope\", \"method\": \"runInScope\", \"file\": \"RequestScope.java\", \"line\": 317, \"exact\": true, \"location\": \"jersey-common-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.server.ServerRuntime\", \"method\": \"process\", \"file\": \"ServerRuntime.java\", \"line\": 305, \"exact\": true, \"location\": \"jersey-server-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.server.ApplicationHandler\", \"method\": \"handle\", \"file\": \"ApplicationHandler.java\", \"line\": 1154, \"exact\": true, \"location\": \"jersey-server-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.servlet.WebComponent\", \"method\": \"serviceImpl\", \"file\": \"WebComponent.java\", \"line\": 473, \"exact\": true, \"location\": \"jersey-container-servlet-core-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.servlet.WebComponent\", \"method\": \"service\", \"file\": \"WebComponent.java\", \"line\": 427, \"exact\": true, \"location\": \"jersey-container-servlet-core-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.servlet.ServletContainer\", \"method\": \"service\", \"file\": \"ServletContainer.java\", \"line\": 388, \"exact\": true, \"location\": \"jersey-container-servlet-core-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.servlet.ServletContainer\", \"method\": \"service\", \"file\": \"ServletContainer.java\", \"line\": 341, \"exact\": true, \"location\": \"jersey-container-servlet-core-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.glassfish.jersey.servlet.ServletContainer\", \"method\": \"service\", \"file\": \"ServletContainer.java\", \"line\": 228, \"exact\": true, \"location\": \"jersey-container-servlet-core-2.25.1.jar\", \"version\": \"?\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"internalDoFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 231, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"doFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 166, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"com.shopstyle.search.query.QueryLoggingServletFilter\", \"method\": \"doFilter\", \"file\": \"QueryLoggingServletFilter.java\", \"line\": 62, \"exact\": true, \"location\": \"search-query-9.0.2.jar\", \"version\": \"?\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"internalDoFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 193, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"doFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 166, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"com.shopstyle.store.StoreServletFilter\", \"method\": \"doFilter\", \"file\": \"StoreServletFilter.java\", \"line\": 100, \"exact\": true, \"location\": \"classes/\", \"version\": \"?\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"internalDoFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 193, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"doFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 166, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"com.shopstyle.common.hibernate.ServletFilter\", \"method\": \"doFilter\", \"file\": \"ServletFilter.java\", \"line\": 59, \"exact\": true, \"location\": \"common-hibernate-32.1.0.jar\", \"version\": \"?\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"internalDoFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 193, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"doFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 166, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"com.shopstyle.common.servlet.HttpProxyFilter\", \"method\": \"doFilter\", \"file\": \"HttpProxyFilter.java\", \"line\": 62, \"exact\": true, \"location\": \"common-servlet-32.1.0.jar\", \"version\": \"?\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"internalDoFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 193, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"doFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 166, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"com.shopstyle.common.servlet.Utf8EncodingFilter\", \"method\": \"doFilter\", \"file\": \"Utf8EncodingFilter.java\", \"line\": 28, \"exact\": true, \"location\": \"common-servlet-32.1.0.jar\", \"version\": \"?\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"internalDoFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 193, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"doFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 166, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.tomcat.websocket.server.WsFilter\", \"method\": \"doFilter\", \"file\": \"WsFilter.java\", \"line\": 52, \"exact\": true, \"location\": \"tomcat-websocket.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"internalDoFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 193, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.core.ApplicationFilterChain\", \"method\": \"doFilter\", \"file\": \"ApplicationFilterChain.java\", \"line\": 166, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.core.StandardWrapperValve\", \"method\": \"invoke\", \"file\": \"StandardWrapperValve.java\", \"line\": 199, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.core.StandardContextValve\", \"method\": \"invoke\", \"file\": \"StandardContextValve.java\", \"line\": 96, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.authenticator.AuthenticatorBase\", \"method\": \"invoke\", \"file\": \"AuthenticatorBase.java\", \"line\": 543, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.core.StandardHostValve\", \"method\": \"invoke\", \"file\": \"StandardHostValve.java\", \"line\": 139, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.valves.ErrorReportValve\", \"method\": \"invoke\", \"file\": \"ErrorReportValve.java\", \"line\": 81, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.core.StandardEngineValve\", \"method\": \"invoke\", \"file\": \"StandardEngineValve.java\", \"line\": 87, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"com.shopstyle.tomcat.RequestStatsValve\", \"method\": \"invoke\", \"file\": \"RequestStatsValve.java\", \"line\": 56, \"exact\": true, \"location\": \"2018-06-25\", \"version\": \"?\" }, { \"class\": \"org.apache.catalina.valves.AbstractAccessLogValve\", \"method\": \"invoke\", \"file\": \"AbstractAccessLogValve.java\", \"line\": 688, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.catalina.connector.CoyoteAdapter\", \"method\": \"service\", \"file\": \"CoyoteAdapter.java\", \"line\": 343, \"exact\": true, \"location\": \"catalina.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.coyote.http11.Http11Processor\", \"method\": \"service\", \"file\": \"Http11Processor.java\", \"line\": 609, \"exact\": true, \"location\": \"tomcat-coyote.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.coyote.AbstractProcessorLight\", \"method\": \"process\", \"file\": \"AbstractProcessorLight.java\", \"line\": 65, \"exact\": true, \"location\": \"tomcat-coyote.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.coyote.AbstractProtocol$ConnectionHandler\", \"method\": \"process\", \"file\": \"AbstractProtocol.java\", \"line\": 818, \"exact\": true, \"location\": \"tomcat-coyote.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.tomcat.util.net.NioEndpoint$SocketProcessor\", \"method\": \"doRun\", \"file\": \"NioEndpoint.java\", \"line\": 1623, \"exact\": true, \"location\": \"tomcat-coyote.jar\", \"version\": \"8.5.51\" }, { \"class\": \"org.apache.tomcat.util.net.SocketProcessorBase\", \"method\": \"run\", \"file\": \"SocketProcessorBase.java\", \"line\": 49, \"exact\": true, \"location\": \"tomcat-coyote.jar\", \"version\": \"8.5.51\" }, { \"class\": \"java.util.concurrent.ThreadPoolExecutor\", \"method\": \"runWorker\", \"file\": \"ThreadPoolExecutor.java\", \"line\": 1128, \"exact\": true, \"location\": \"?\", \"version\": \"?\" }, { \"class\": \"java.util.concurrent.ThreadPoolExecutor$Worker\", \"method\": \"run\", \"file\": \"ThreadPoolExecutor.java\", \"line\": 628, \"exact\": true, \"location\": \"?\", \"version\": \"?\" }, { \"class\": \"org.apache.tomcat.util.threads.TaskThread$WrappingRunnable\", \"method\": \"run\", \"file\": \"TaskThread.java\", \"line\": 61, \"exact\": true, \"location\": \"tomcat-util.jar\", \"version\": \"8.5.51\" }, { \"class\": \"java.lang.Thread\", \"method\": \"run\", \"file\": \"Thread.java\", \"line\": 834, \"exact\": true, \"location\": \"?\", \"version\": \"?\" } ] }, \"endOfBatch\": false, \"loggerFqcn\": \"org.apache.logging.slf4j.Log4jLogger\", \"contextMap\": { \"pid\": \"shopstyle (385837)\", \"userId\": \"43370716\" }, \"threadId\": 139, \"threadPriority\": 5 } ";

//         let result = create_log_string("INFO", false, false, input);


// }