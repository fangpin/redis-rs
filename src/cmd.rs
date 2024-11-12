use crate::{error::DBError, protocol::Protocol, server::Server};

#[derive(Debug)]
pub enum Cmd {
    Ping,
    Echo(String),
    Get(String),
    Set(String, String),
    SetPx(String, String, u128),
    SetEx(String, String, u128),
    Keys,
    ConfigGet(String),
    Info(Option<String>),
    Del(String),
    Replconf(String, String),
    Psync(String, String),
    Type(String),
}

impl Cmd {
    pub fn from(s: &str) -> Result<(Self, Protocol), DBError> {
        let protocol = Protocol::from(s)?;
        match protocol.clone().0 {
            Protocol::Array(p) => {
                let cmd = p.into_iter().map(|x| x.decode()).collect::<Vec<_>>();
                if cmd.len() == 0 {
                    return Err(DBError("cmd length is 0".to_string()));
                }
                Ok((
                    match cmd[0].as_str() {
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
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                            } else {
                                Cmd::ConfigGet(cmd[2].clone())
                            }
                        }
                        "keys" => {
                            if cmd.len() != 2 || cmd[1] != "*" {
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                            } else {
                                Cmd::Keys
                            }
                        }
                        "info" => {
                            let section = if cmd.len() == 2 {
                                Some(cmd[1].clone())
                            } else {
                                None
                            };
                            Cmd::Info(section)
                        }
                        "replconf" => {
                            if cmd.len() < 3 {
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                            }
                            Cmd::Replconf(cmd[1].clone(), cmd[2].clone())
                        }
                        "psync" => {
                            if cmd.len() != 3 {
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                            }
                            Cmd::Psync(cmd[1].clone(), cmd[2].clone())
                        }
                        "del" => {
                            if cmd.len() != 2 {
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                            }
                            Cmd::Del(cmd[1].clone())
                        }
                        "type" => {
                            if cmd.len() != 2 {
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                            }
                            Cmd::Type(cmd[1].clone())
                        }
                        _ => return Err(DBError(format!("unknown cmd {:?}", cmd[0]))),
                    },
                    protocol.0,
                ))
            }
            _ => Err(DBError(format!(
                "fail to parse as cmd for {:?}",
                protocol.0
            ))),
        }
    }

    pub async fn run(
        self: &Self,
        server: &mut Server,
        protocol: Protocol,
        is_rep_con: bool,
    ) -> Result<Protocol, DBError> {
        // return if the command is a write command
        let p = protocol.clone();
        let ret = match self {
            Cmd::Ping => Ok(Protocol::SimpleString("PONG".to_string())),
            Cmd::Echo(s) => Ok(Protocol::SimpleString(s.clone())),
            Cmd::Get(k) => {
                let v = {
                    let mut s = server.storage.lock().await;
                    s.get(k)
                };
                Ok(v.map_or(Protocol::Null, Protocol::SimpleString))
            }
            Cmd::Set(k, v) => {
                let offset = {
                    let mut s = server.storage.lock().await;
                    s.set(k.clone(), v.clone());
                    server
                        .offset
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                        + 1
                };
                if server.is_master() {
                    server
                        .master_repl_clients
                        .lock()
                        .await
                        .as_mut()
                        .unwrap()
                        .send_command(protocol)
                        .await?;
                    Ok(Protocol::ok())
                } else if !is_rep_con {
                    Ok(Protocol::write_on_slave_err())
                } else {
                    Ok(Protocol::ok())
                }
            }
            Cmd::SetPx(k, v, x) => {
                let offset = {
                    let mut s = server.storage.lock().await;
                    s.setx(k.clone(), v.clone(), *x);
                    server
                        .offset
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                };
                if server.is_master() {
                    server
                        .master_repl_clients
                        .lock()
                        .await
                        .as_mut()
                        .unwrap()
                        .send_command(protocol)
                        .await?;
                    Ok(Protocol::ok())
                } else if !is_rep_con {
                    Ok(Protocol::write_on_slave_err())
                } else {
                    Ok(Protocol::ok())
                }
            }
            Cmd::SetEx(k, v, x) => {
                let offset = {
                    let mut s = server.storage.lock().await;
                    s.setx(k.clone(), v.clone(), *x * 1000);
                    server
                        .offset
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                };
                if server.is_master() {
                    server
                        .master_repl_clients
                        .lock()
                        .await
                        .as_mut()
                        .unwrap()
                        .send_command(protocol)
                        .await?;
                    Ok(Protocol::ok())
                } else if !is_rep_con {
                    Ok(Protocol::write_on_slave_err())
                } else {
                    Ok(Protocol::ok())
                }
            }
            Cmd::Del(k) => {
                let offset = {
                    let mut s = server.storage.lock().await;
                    s.del(k.clone());
                    server
                        .offset
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                };
                if server.is_master() {
                    server
                        .master_repl_clients
                        .lock()
                        .await
                        .as_mut()
                        .unwrap()
                        .send_command(protocol)
                        .await?;
                    Ok(Protocol::ok())
                } else if !is_rep_con {
                    Ok(Protocol::write_on_slave_err())
                } else {
                    Ok(Protocol::ok())
                }
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
                _ => Err(DBError(format!("unsupported config {:?}", name))),
            },
            Cmd::Keys => {
                let keys = { server.storage.lock().await.keys() };
                Ok(Protocol::Array(
                    keys.into_iter().map(Protocol::BulkString).collect(),
                ))
            }
            Cmd::Info(section) => match section {
                Some(s) => match s.as_str() {
                    "replication" => Ok(Protocol::BulkString(format!(
                        "role:{}\nmaster_replid:{}\nmaster_repl_offset:{}\n",
                        server.option.replication.role,
                        server.option.replication.master_replid,
                        server.option.replication.master_repl_offset
                    ))),
                    _ => Err(DBError(format!("unsupported section {:?}", s))),
                },
                None => Ok(Protocol::BulkString(format!("default"))),
            },
            Cmd::Replconf(sub_cmd, _) => match sub_cmd.as_str() {
                "getack" => Ok(Protocol::form_vec(vec![
                    "REPLCONF",
                    "ACK",
                    server
                        .offset
                        .load(std::sync::atomic::Ordering::Relaxed)
                        .to_string()
                        .as_str(),
                ])),
                _ => Ok(Protocol::SimpleString("OK".to_string())),
            },
            Cmd::Psync(_, _) => {
                if server.is_master() {
                    Ok(Protocol::SimpleString(format!(
                        "FULLRESYNC {} 0",
                        server.option.replication.master_replid
                    )))
                } else {
                    Ok(Protocol::psync_on_slave_err())
                }
            }
            Cmd::Type(k) => {
                let v = { server.storage.lock().await.get(k) };
                Ok(v.map_or(Protocol::none(), |_| {
                    Protocol::SimpleString("string".to_string())
                }))
            }
        };
        if ret.is_ok() {
            server.offset.fetch_add(
                p.encode().len() as u64,
                std::sync::atomic::Ordering::Relaxed,
            );
        }
        ret
    }
}
