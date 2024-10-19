use crate::{protocol::Protocol, server::Server};
use anyhow::Result;

pub enum Cmd {
    Ping,
    Echo(String),
    Get(String),
    Set(String, String),
    SetPx(String, String, u128),
    SetEx(String, String, u128),
    ConfigGet(String),
}

impl Cmd {
    pub fn from(s: &str) -> Result<Self> {
        let protocol = Protocol::from(s)?;
        match protocol.0 {
            Protocol::Array(p) => {
                let cmd = p.into_iter().map(|x| x.decode()).collect::<Vec<_>>();
                if cmd.len() == 0 {
                    return Err(anyhow::anyhow!("cmd length is 0"));
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
                            return Err(anyhow::anyhow!("unsupported cmd {:?}", cmd));
                        } else {
                            Cmd::ConfigGet(cmd[2].clone())
                        }
                    }
                    _ => return Err(anyhow::anyhow!("unknown cmd {:?}", cmd[0])),
                })
            }
            _ => Err(anyhow::anyhow!("fail to parse as cmd for {:?}", protocol.0)),
        }
    }

    pub fn run(self: &Self, server: &mut Server) -> Result<Protocol> {
        match self {
            Cmd::Ping => Ok(Protocol::SimpleString("PONG".to_string())),
            Cmd::Echo(s) => Ok(Protocol::SimpleString(s.clone())),
            Cmd::Get(k) => {
                let s = server.storage.lock().unwrap();
                Ok(if let Some(v) = s.get(k) {
                    Protocol::SimpleString(v.clone())
                } else {
                    Protocol::Null
                })
            }
            Cmd::Set(k, v) => {
                {
                    let mut s = server.storage.lock().unwrap();
                    s.set(k.clone(), v.clone());
                }
                Ok(Protocol::ok())
            }
            Cmd::SetPx(k, v, x) => {
                {
                    let mut s = server.storage.lock().unwrap();
                    s.setx(k.clone(), v.clone(), *x);
                }
                Ok(Protocol::ok())
            }
            Cmd::SetEx(k, v, x) => {
                {
                    let mut s = server.storage.lock().unwrap();
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
                _ => Err(anyhow::anyhow!("unsupported config {:?}", name)),
            },
        }
    }
}
