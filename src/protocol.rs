use std::{any, error::Error};
use anyhow::Result;

#[derive(Debug)]
pub enum Protocol {
    SimpleString(String),
    BulkString(String),
    Array(Vec<Protocol>),
}

impl Protocol {
    pub fn from(protocol: &str) -> Result<(Self, usize)> {
        match protocol.chars().nth(0) {
            Some('+') => Self::new_simple_string(&protocol[1..]),
            Some('$') => Self::new_bulk_string(&protocol[1..]),
            Some('*') => Self::new_array(&protocol[1..]),
            _ => Err(anyhow::anyhow!("[from] unsupported protocol: {:?}", protocol)),
        }
    }

    pub fn decode(self: &Self) -> String {
        match self {
            Protocol::SimpleString(s) => s.to_string(),
            Protocol::BulkString(s) => s.to_string(),
            Protocol::Array(s) => s.into_iter().map(|x| x.decode()).collect::<Vec<_>>().join(" "),
        }
    }

    pub fn encode(self: &Self) -> String {
        match self {
            Protocol::SimpleString(s) => format!("+{}\\r\\n", s),
            Protocol::BulkString(s) => format!("${}\\r\\n{}\\r\\n", s.len(), s),
            Protocol::Array(ss) => format!("${}\\r\\n", ss.len()) + ss.into_iter().map(|x| x.encode()).collect::<Vec<_>>().join("").as_str(),
        }
    }

    fn new_simple_string(protocol: &str) -> Result<(Self, usize)> {
        match protocol.find("\r\n") {
            Some(x) => Ok((Self::SimpleString(protocol[..x].to_string()), x + 2 + 1)),
            _ => Err(anyhow::anyhow!(format!("[new simple string] unsupported protocol: {:?}", protocol))),
        }
    }

    fn new_bulk_string(protocol: &str) -> Result<(Self, usize)> {
        if let Some(len) = protocol.find("\r\n") {
            let size = Self::parse_usize(&protocol[..len])?;
            if let Some(data_len) = protocol[len+2..].find("\r\n") {
                let s = Self::parse_string(&protocol[len+2..])?;
                if size != s.len() {
                    Err(anyhow::anyhow!("[new bulk string] unmatched string length in prototocl {:?}", protocol))
                } else {
                    Ok((Protocol::BulkString(s), len + 2 + data_len + 2 + 1))
                }
            } else {
                Err(anyhow::anyhow!("[new bulk string] unsupported protocol: {:?}", protocol))
            }
        } else {
            Err(anyhow::anyhow!("[new bulk string] unsupported protocol: {:?}", protocol))
        }
    }

    fn new_array(protocol: &str) -> Result<(Self, usize)> {
        match protocol.find("\r\n") {
            Some(x) => {
                let num = protocol[..x].parse::<usize>()?;
                match protocol[x+2..].find("\r\n") {
                    Some(len) => {
                        let mut array = vec![];
                        let mut offset = x + 2;
                        let mut total = x + 2;
                        while let Ok((p, l)) = Self::from(&protocol[offset..offset+len+2]) {
                            offset += l;
                            total += l;
                            array.push(p);
                        }
                        if array.len() != num {
                            return Err(anyhow::anyhow!("[new array] unmatched array length in protocol {:?}", protocol));
                        }
                        Ok((Self::Array(array), total + 1))
                    },
                    _ => Err(anyhow::anyhow!("[new array] unsupported protocol: {:?}", protocol)),
                }
            }
            _ => Err(anyhow::anyhow!("[new array] unsupported protocol: {:?}", protocol))
        }
    }

    fn parse_usize(protocol: &str) -> Result<usize> {
        match protocol.len() {
            0 => Err(anyhow::anyhow!("parse usize error: {:?}", protocol)),
            _ => Ok(protocol.parse::<usize>()?),
        }
    }

    fn parse_string(protocol: &str) -> Result<String> {
        match protocol.len() {
            0 => Err(anyhow::anyhow!("parse usize error: {:?}", protocol)),
            _ => Ok(protocol.to_string()),
        }
    }
}