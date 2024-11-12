use crate::error::DBError;

#[derive(Debug, Clone)]
pub enum Protocol {
    SimpleString(String),
    BulkString(String),
    Null,
    Array(Vec<Protocol>),
}

impl Protocol {
    pub fn from(protocol: &str) -> Result<(Self, usize), DBError> {
        let ret = match protocol.chars().nth(0) {
            Some('+') => Self::parse_simple_string_sfx(&protocol[1..]),
            Some('$') => Self::parse_bulk_string_sfx(&protocol[1..]),
            Some('*') => Self::parse_array_sfx(&protocol[1..]),
            _ => Err(DBError(format!(
                "[from] unsupported protocol: {:?}",
                protocol
            ))),
        };
        match ret {
            Ok((p, s)) => Ok((p, s + 1)),
            Err(e) => Err(e),
        }
    }

    pub fn form_vec(array: Vec<&str>) -> Self {
        let array = array
            .into_iter()
            .map(|x| Protocol::BulkString(x.to_string()))
            .collect();
        Protocol::Array(array)
    }

    #[inline]
    pub fn ok() -> Self {
        Protocol::SimpleString("ok".to_string())
    }

    #[inline]
    pub fn err(msg: &str) -> Self {
        Protocol::SimpleString(msg.to_string())
    }

    #[inline]
    pub fn write_on_slave_err() -> Self {
        Self::err("DISALLOW WRITE ON SLAVE")
    }

    #[inline]
    pub fn psync_on_slave_err() -> Self {
        Self::err("PSYNC ON SLAVE IS NOT ALLOWED")
    }

    #[inline]
    pub fn none() -> Self {
        Self::SimpleString("none".to_string())
    }

    pub fn decode(self: &Self) -> String {
        match self {
            Protocol::SimpleString(s) => s.to_string(),
            Protocol::BulkString(s) => s.to_string(),
            Protocol::Null => "".to_string(),
            Protocol::Array(s) => s
                .into_iter()
                .map(|x| x.decode())
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    pub fn to_string(self: &Self) -> String {
        self.decode()
    }

    pub fn encode(self: &Self) -> String {
        match self {
            Protocol::SimpleString(s) => format!("+{}\r\n", s),
            Protocol::BulkString(s) => format!("${}\r\n{}\r\n", s.len(), s),
            Protocol::Array(ss) => {
                format!("*{}\r\n", ss.len())
                    + ss.into_iter()
                        .map(|x| x.encode())
                        .collect::<Vec<_>>()
                        .join("")
                        .as_str()
            }
            Protocol::Null => "$-1\r\n".to_string(),
        }
    }

    fn parse_simple_string_sfx(protocol: &str) -> Result<(Self, usize), DBError> {
        match protocol.find("\r\n") {
            Some(x) => Ok((Self::SimpleString(protocol[..x].to_string()), x + 2)),
            _ => Err(DBError(format!(
                "[new simple string] unsupported protocol: {:?}",
                protocol
            ))),
        }
    }

    fn parse_bulk_string_sfx(protocol: &str) -> Result<(Self, usize), DBError> {
        if let Some(len) = protocol.find("\r\n") {
            let size = Self::parse_usize(&protocol[..len])?;
            if let Some(data_len) = protocol[len + 2..].find("\r\n") {
                let s = Self::parse_string(&protocol[len + 2..len + 2 + data_len])?;
                if size != s.len() {
                    Err(DBError(format!(
                        "[new bulk string] unmatched string length in prototocl {:?}",
                        protocol,
                    )))
                } else {
                    Ok((
                        Protocol::BulkString(s.to_lowercase()),
                        len + 2 + data_len + 2,
                    ))
                }
            } else {
                Err(DBError(format!(
                    "[new bulk string] unsupported protocol: {:?}",
                    protocol
                )))
            }
        } else {
            Err(DBError(format!(
                "[new bulk string] unsupported protocol: {:?}",
                protocol
            )))
        }
    }

    fn parse_array_sfx(s: &str) -> Result<(Self, usize), DBError> {
        let mut offset = 0;
        match s.find("\r\n") {
            Some(x) => {
                let array_len = s[..x].parse::<usize>()?;
                offset += x + 2;
                let mut vec = vec![];
                for _ in 0..array_len {
                    match Protocol::from(&s[offset..]) {
                        Ok((p, len)) => {
                            offset += len;
                            vec.push(p);
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
                Ok((Protocol::Array(vec), offset))
            }
            _ => Err(DBError(format!(
                "[new array] unsupported protocol: {:?}",
                s
            ))),
        }
    }

    fn parse_usize(protocol: &str) -> Result<usize, DBError> {
        match protocol.len() {
            0 => Err(DBError(format!("parse usize error: {:?}", protocol))),
            _ => Ok(protocol
                .parse::<usize>()
                .map_err(|_| DBError(format!("parse usize error: {}", protocol)))?),
        }
    }

    fn parse_string(protocol: &str) -> Result<String, DBError> {
        match protocol.len() {
            0 => Err(DBError(format!("parse usize error: {:?}", protocol))),
            _ => Ok(protocol.to_string()),
        }
    }
}
