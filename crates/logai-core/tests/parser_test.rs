use logai_core::parser::{ApacheParser, NginxParser, SyslogParser, LogParser};

#[test]
fn test_apache_parser(){
    let parser = ApacheParser::new();

    // Valid Apache error log test1

    let raw = "[Sun Dec 04 04:47:44 2005] [error] mod_jk child workerEnv in error state 6";
    let result = parser.parse(raw);

    assert!(result.is_ok());
    let entry = result.unwrap();

    println!("Message: {}", entry.message);
    println!("Level: {:?}", entry.level);
    println!("Timestamp: {:?}", entry.timestamp);
    println!("Service: {:?}", entry.service);

    assert_eq!(entry.message, "mod_jk child workerEnv in error state 6");
    assert!(entry.level.is_some());
    assert!(entry.timestamp.is_some());
}

#[test]
fn test_apache_parser_fallback() {
    let parser = ApacheParser::new();

    // Invalid format Fallback test2
    let raw = "random text that doesnt match";
    let result = parser.parse(raw);

    assert!(result.is_ok());
    let entry = result.unwrap();

    assert_eq!(entry.message, raw);
    assert!(entry.timestamp.is_none()); // could not parse
}

#[test]
fn test_loghub_apache_logs() {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let parser = ApacheParser::new();

    // read first 100 lines from LogHub
    let file = File::open("../../sample-data/Apache.log").expect("Loghub file not found");
    let reader = BufReader::new(file);

    let mut success = 0;
    let mut failed = 0;
    let mut errors = 0;
    let mut notices = 0;
    
    for (i, line) in reader.lines().take(100).enumerate() {
        let raw = line.unwrap();
        match parser.parse(&raw) {
            Ok(entry) => {
                success += 1;
                if entry.timestamp.is_some(){
                    // count by level
                    match entry.level {
                        Some(logai_core::LogLevel::Error) => errors += 1,
                        Some(logai_core::LogLevel::Info) => notices += 1, // notice maps to info
                        _ => {}
                    }
                }
            }
            Err(e) => {
                failed += 1;
                println!("Line {}: Failed - {}", i, e.message);
            }
        }
    }
    println!("\n=== Loghub Apache Test Results ===");
    println!("Total parsed: {}", success);
    println!("Failed: {}", failed);
    println!("Errors found: {}", errors);
    println!("Notices found: {}", notices);

    assert!(success > 90, "At least 90% should parse");

}

// ============ NGINX PARSER TESTS ============

#[test]
fn test_nginx_error_log() {
    let parser = NginxParser::new();

    // Nginx error log format
    let raw = "2024/02/08 10:30:00 [error] 12345#0: *1234 open() \"/var/www/html/favicon.ico\" failed";
    let result = parser.parse(raw);

    assert!(result.is_ok());
    let entry = result.unwrap();

    println!("Nginx Error - Message: {}", entry.message);
    println!("Level: {:?}", entry.level);
    println!("Timestamp: {:?}", entry.timestamp);

    assert!(entry.message.contains("favicon.ico"));
    assert!(entry.level.is_some());
    assert!(entry.timestamp.is_some());
    assert_eq!(entry.fields.get("pid").unwrap(), &serde_json::json!("12345"));
}

#[test]
fn test_nginx_access_log() {
    let parser = NginxParser::new();

    // Nginx access log (combined format)
    let raw = r#"192.168.1.1 - - [08/Feb/2024:10:30:00 +0000] "GET /api/users HTTP/1.1" 200 1234"#;
    let result = parser.parse(raw);

    assert!(result.is_ok());
    let entry = result.unwrap();

    println!("Nginx Access - Message: {}", entry.message);
    println!("Level: {:?}", entry.level);
    println!("Fields: {:?}", entry.fields);

    assert!(entry.message.contains("GET"));
    assert_eq!(entry.fields.get("status").unwrap(), &serde_json::json!(200));
    assert_eq!(entry.fields.get("ip").unwrap(), &serde_json::json!("192.168.1.1"));
}

#[test]
fn test_nginx_access_error_status() {
    let parser = NginxParser::new();

    // 500 error should be LogLevel::Error
    let raw = r#"10.0.0.1 - - [08/Feb/2024:10:30:00 +0000] "POST /api/crash HTTP/1.1" 500 0"#;
    let result = parser.parse(raw);

    assert!(result.is_ok());
    let entry = result.unwrap();

    assert_eq!(entry.level, Some(logai_core::LogLevel::Error));
    assert_eq!(entry.fields.get("status").unwrap(), &serde_json::json!(500));
}

// ============ SYSLOG PARSER TESTS ============

#[test]
fn test_syslog_bsd_format() {
    let parser = SyslogParser::new();

    // BSD syslog format with priority
    let raw = "<34>Oct 11 22:14:15 mymachine su[12345]: 'su root' failed for user";
    let result = parser.parse(raw);

    assert!(result.is_ok());
    let entry = result.unwrap();

    println!("Syslog - Message: {}", entry.message);
    println!("Level: {:?}", entry.level);
    println!("Service: {:?}", entry.service);
    println!("Fields: {:?}", entry.fields);

    assert!(entry.message.contains("su root"));
    assert_eq!(entry.service, Some("su".to_string()));
    assert_eq!(entry.fields.get("hostname").unwrap(), &serde_json::json!("mymachine"));
    assert_eq!(entry.fields.get("pid").unwrap(), &serde_json::json!("12345"));
}

#[test]
fn test_syslog_without_priority() {
    let parser = SyslogParser::new();

    // Without priority tag
    let raw = "Feb 08 14:30:00 webserver nginx[1234]: connection closed";
    let result = parser.parse(raw);

    assert!(result.is_ok());
    let entry = result.unwrap();

    println!("Syslog no-priority - Message: {}", entry.message);
    assert!(entry.message.contains("connection"));
    assert!(entry.timestamp.is_some());
}

#[test]
fn test_syslog_priority_to_level() {
    let parser = SyslogParser::new();

    // Priority 11 = facility 1, severity 3 (Error)
    let raw = "<11>Feb 08 10:00:00 host app: critical error occurred";
    let result = parser.parse(raw);

    assert!(result.is_ok());
    let entry = result.unwrap();

    assert_eq!(entry.level, Some(logai_core::LogLevel::Error));
}
#[test]
fn test_loghub_syslog_sample() {
    let parser = SyslogParser::new();
    
    // Sample from Loghub Linux dataset
    let raw = "Jun 14 15:16:01 combo sshd(pam_unix)[19939]: authentication failure; logname= uid=0 euid=0";
    let result = parser.parse(raw);
    
    assert!(result.is_ok(), "Should parse Loghub syslog format");
    let entry = result.unwrap();
    
    println!("Loghub Syslog - Message: {}", entry.message);
    println!("Service: {:?}", entry.service);
    println!("Timestamp: {:?}", entry.timestamp);
    
    assert!(entry.message.contains("authentication failure"));
}
