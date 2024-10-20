use crate::{error::Error, protocol::Protocol, server::Server};

pub enum Cmd {
    Ping,
    Echo(String),
    Get(String),
    Set(String, String),
    SetPx(String, String, u128),
    SetEx(String, String, u128),
    Keys,
    ConfigGet(String),
}

impl Cmd {
    pub fn from(s: &str) -> Result<Self, Error> {
        let protocol = Protocol::from(s)?;
        match protocol.0 {
            Protocol::Array(p) => {
                let cmd = p.into_iter().map(|x| x.decode()).collect::<Vec<_>>();
                if cmd.len() == 0 {
                    return Err(Error::E("cmd length is 0".to_string()));
                }
                Ok(match cmd[0].as_str() {
                    "echo" => Cmd::Echo(cmd[1].clone()),
                    "ping" => Cmd::Ping,
                    "get" => Cmd::Get(cmd[1].clone()),
                    "set" => {
                        if cmd.len() == 5 && cmd[3] == "px" {
                            Cmd::SetPx(cmd[1].clone(), cmd[2].clone(), cmd[4].parse().unwrap())
                        } else if cmd.len() == 5 && cmd[3] == "ex" {
                            Cmd::SetEx(cmd[1].clone(), cmd[2].clone(), cmd[4].parse().unwrap())
                        } else {
                            Cmd::Set(cmd[1].clone(), cmd[2].clone())
                        }
                    }
                    "config" => {
                        if cmd.len() != 3 || cmd[1] != "get" {
                            return Err(Error::E(format!("unsupported cmd {:?}", cmd)));
                        } else {
                            Cmd::ConfigGet(cmd[2].clone())
                        }
                    }
                    "keys" => {
                        if cmd.len() != 2 || cmd[1] != "*" {
                            return Err(Error::E(format!("unsupported cmd {:?}", cmd)));
                        } else {
                            Cmd::Keys
                        }
                    }
                    _ => return Err(Error::E(format!("unknown cmd {:?}", cmd[0]))),
                })
            }
            _ => Err(Error::E(format!(
                "fail to parse as cmd for {:?}",
                protocol.0
            ))),
        }
    }

    pub fn run(self: &Self, server: &mut Server) -> Result<Protocol, String> {
        match self {
            Cmd::Ping => Ok(Protocol::SimpleString("PONG".to_string())),
            Cmd::Echo(s) => Ok(Protocol::SimpleString(s.clone())),
            Cmd::Get(k) => {
                let s = server.storage.read().unwrap();
                Ok(if let Some(v) = s.get(k) {
                    Protocol::SimpleString(v.clone())
                } else {
                    Protocol::Null
                })
            }
            Cmd::Set(k, v) => {
                {
                    let mut s = server.storage.write().unwrap();
                    s.set(k.clone(), v.clone());
                }
                Ok(Protocol::ok())
            }
            Cmd::SetPx(k, v, x) => {
                {
                    let mut s = server.storage.write().unwrap();
                    s.setx(k.clone(), v.clone(), *x);
                }
                Ok(Protocol::ok())
            }
            Cmd::SetEx(k, v, x) => {
                {
                    let mut s = server.storage.write().unwrap();
                    s.setx(k.clone(), v.clone(), *x * 1000);
                }
                Ok(Protocol::ok())
            }
            Cmd::ConfigGet(name) => match name.as_str() {
                "dir" => Ok(Protocol::Array(vec![
                    Protocol::BulkString(name.clone()),
                    Protocol::BulkString(server.option.dir.clone()),
                ])),
                "dbfilename" => Ok(Protocol::Array(vec![
                    Protocol::BulkString(name.clone()),
                    Protocol::BulkString(server.option.db_file_name.clone()),
                ])),
                _ => Err(format!("unsupported config {:?}", name)),
            },
            Cmd::Keys => {
                let keys = { server.storage.read().unwrap().keys() };
                Ok(Protocol::Array(
                    keys.into_iter().map(|x| Protocol::BulkString(x)).collect(),
                ))
            }
        }
    }
}
