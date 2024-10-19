use std::sync::{Arc, Mutex};

use crate::{protocol::Protocol, storage::Storage};
use anyhow::Result;

pub enum Cmd {
    Ping,
    Echo(String),
    Get(String),
    Set(String, String),
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
                    "set" => Cmd::Set(cmd[1].clone(), cmd[2].clone()),
                    _ => return Err(anyhow::anyhow!("unknown cmd {:?}", cmd[0])),
                })
            }
            _ => Err(anyhow::anyhow!("fail to parse as cmd for {:?}", protocol.0)),
        }
    }

    pub fn run(self: &Self, storage: &mut Arc<Mutex<Storage>>) -> Result<Protocol> {
        match self {
            Cmd::Ping => Ok(Protocol::SimpleString("PONG".to_string())),
            Cmd::Echo(s) => Ok(Protocol::SimpleString(s.clone())),
            Cmd::Get(k) => {
                let s = storage.lock().unwrap();
                Ok(if let Some(v) = s.get(k) {
                    Protocol::SimpleString(v.clone())
                } else {
                    Protocol::Null
                })
            },
            Cmd::Set(k, v) => {
                {
                    let mut s = storage.lock().unwrap();
                    s.set(k.clone(), v.clone());
                }
                Ok(Protocol::ok())
            }
        }
    }
}
