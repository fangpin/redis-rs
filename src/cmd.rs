use crate::protocol::Protocol;
use anyhow::Result;

pub enum Cmd {
    Ping,
    Echo(String),
    Unknown(),
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
                    _ => Cmd::Unknown(),
                })
            }
            _ => Err(anyhow::anyhow!("fail to parse as cmd for {:?}", protocol.0)),
        }
    }

    pub fn run(self: &Self) -> Result<Protocol> {
        match self {
            Cmd::Echo(s) => Ok(Protocol::SimpleString(s.clone())),
            Cmd::Ping => Ok(Protocol::SimpleString("PONG".to_string())),
            _ => Err(anyhow::anyhow!("unknown cmd")),
        }
    }
}
