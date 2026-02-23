//! log parser registry - parse raw logs into structured format

pub mod apache;
pub mod nginx;
pub mod proxmox;
pub mod syslog;

pub use apache::ApacheParser;
pub use nginx::NginxParser;
pub use proxmox::ProxmoxParser;
pub use syslog::SyslogParser;

use crate::RawLogEntry;
use std::{collections::HashMap};

//parse error type
#[derive(Debug)]
pub struct ParseError{
    pub message: String,
}

impl ParseError {
    pub fn new(msg: &str) -> Self {
        Self { message: msg.to_string() }
    }
}

// Parser trait - every parser implement this

pub trait LogParser: Send + Sync {
    fn name(&self) -> &'static str; 
    fn parse(&self, raw: &str) -> Result<RawLogEntry, ParseError>; 
}

// Registry to hold all parsers

pub struct ParserRegistry {
    parsers: HashMap<String, Box<dyn LogParser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        Self { parsers: HashMap::new(), }
    }

    // register a parser
    pub fn register(&mut self, parser: Box<dyn LogParser>) {
        self.parsers.insert(parser.name().to_string(), parser);
    
    }

    // Get parser by name
    pub fn get(&self, name: &str) -> Option<&dyn LogParser> {
        self.parsers.get(name).map(|p| p.as_ref())
    }

    //parse using speicified format
    pub fn parse(&self, format: &str, raw: &str) -> Result<RawLogEntry, ParseError> {
        match self.get(format) {
            Some(parser) => parser.parse(raw),
            None => Err(ParseError::new(&format!("Unknown format: {}", format))),
        }
    }
}