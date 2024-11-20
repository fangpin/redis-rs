use std::{collections::BTreeMap, ops::Bound, time::Duration, u64};

use tokio::sync::mpsc;

use crate::{error::DBError, protocol::Protocol, server::Server, storage::now_in_millis};

#[derive(Debug, Clone)]
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
    Replconf(String),
    Psync,
    Type(String),
    Xadd(String, String, Vec<(String, String)>),
    Xrange(String, String, String),
    Xread(Vec<String>, Vec<String>, Option<u64>),
    Incr(String),
    Multi,
    Exec,
    Unknow,
    Discard,
}

impl Cmd {
    pub fn from(s: &str) -> Result<(Self, Protocol), DBError> {
        let protocol = Protocol::from(s)?;
        match protocol.clone().0 {
            Protocol::Array(p) => {
                let cmd = p.into_iter().map(|x| x.decode()).collect::<Vec<_>>();
                if cmd.is_empty() {
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
                            } else if cmd.len() == 3 {
                                Cmd::Set(cmd[1].clone(), cmd[2].clone())
                            } else {
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
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
                            Cmd::Replconf(cmd[1].clone())
                        }
                        "psync" => {
                            if cmd.len() != 3 {
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                            }
                            Cmd::Psync
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
                        "xadd" => {
                            if cmd.len() < 5 {
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                            }

                            let mut key_value = Vec::<(String, String)>::new();
                            let mut i = 3;
                            while i < cmd.len() - 1 {
                                key_value.push((cmd[i].clone(), cmd[i + 1].clone()));
                                i += 2;
                            }
                            Cmd::Xadd(cmd[1].clone(), cmd[2].clone(), key_value)
                        }
                        "xrange" => {
                            if cmd.len() != 4 {
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                            }
                            Cmd::Xrange(cmd[1].clone(), cmd[2].clone(), cmd[3].clone())
                        }
                        "xread" => {
                            if cmd.len() < 4 || cmd.len() % 2 != 0 {
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                            }
                            let mut offset = 2;
                            // block cmd
                            let mut block = None;
                            if cmd[1] == "block" {
                                offset += 2;
                                if let Ok(block_time) = cmd[2].parse() {
                                    block = Some(block_time);
                                } else {
                                    return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                                }
                            }
                            let cmd2 = &cmd[offset..];
                            let len2 = cmd2.len() / 2;
                            Cmd::Xread(cmd2[0..len2].to_vec(), cmd2[len2..].to_vec(), block)
                        }
                        "incr" => {
                            if cmd.len() != 2 {
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                            }
                            Cmd::Incr(cmd[1].clone())
                        }
                        "multi" => {
                            if cmd.len() != 1 {
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                            }
                            Cmd::Multi
                        }
                        "exec" => {
                            if cmd.len() != 1 {
                                return Err(DBError(format!("unsupported cmd {:?}", cmd)));
                            }
                            Cmd::Exec
                        }
                        "discard" => Cmd::Discard,
                        _ => Cmd::Unknow,
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
        &self,
        server: &mut Server,
        protocol: Protocol,
        is_rep_con: bool,
        queued_cmd: &mut Option<Vec<(Cmd, Protocol)>>,
    ) -> Result<Protocol, DBError> {
        // return if the command is a write command
        let p = protocol.clone();
        if queued_cmd.is_some()
            && !matches!(self, Cmd::Exec)
            && !matches!(self, Cmd::Multi)
            && !matches!(self, Cmd::Discard)
        {
            queued_cmd
                .as_mut()
                .unwrap()
                .push((self.clone(), protocol.clone()));
            return Ok(Protocol::SimpleString("QUEUED".to_string()));
        }
        let ret = match self {
            Cmd::Ping => Ok(Protocol::SimpleString("PONG".to_string())),
            Cmd::Echo(s) => Ok(Protocol::SimpleString(s.clone())),
            Cmd::Get(k) => get_cmd(server, k).await,
            Cmd::Set(k, v) => set_cmd(server, k, v, protocol, is_rep_con).await,
            Cmd::SetPx(k, v, x) => set_px_cmd(server, k, v, x, protocol, is_rep_con).await,
            Cmd::SetEx(k, v, x) => set_ex_cmd(server, k, v, x, protocol, is_rep_con).await,
            Cmd::Del(k) => del_cmd(server, k, protocol, is_rep_con).await,
            Cmd::ConfigGet(name) => config_get_cmd(name, server),
            Cmd::Keys => keys_cmd(server).await,
            Cmd::Info(section) => info_cmd(section, server),
            Cmd::Replconf(sub_cmd) => replconf_cmd(sub_cmd, server),
            Cmd::Psync => psync_cmd(server),
            Cmd::Type(k) => type_cmd(server, k).await,
            Cmd::Xadd(stream_key, offset, kvps) => {
                xadd_cmd(
                    offset.as_str(),
                    server,
                    stream_key.as_str(),
                    kvps,
                    protocol,
                    is_rep_con,
                )
                .await
            }

            Cmd::Xrange(stream_key, start, end) => xrange_cmd(server, stream_key, start, end).await,
            Cmd::Xread(stream_keys, starts, block) => {
                xread_cmd(starts, server, stream_keys, block).await
            }
            Cmd::Incr(key) => incr_cmd(server, key).await,
            Cmd::Multi => {
                *queued_cmd = Some(Vec::<(Cmd, Protocol)>::new());
                Ok(Protocol::SimpleString("ok".to_string()))
            }
            Cmd::Exec => exec_cmd(queued_cmd, server, is_rep_con).await,
            Cmd::Discard => {
                if queued_cmd.is_some() {
                    *queued_cmd = None;
                    Ok(Protocol::SimpleString("ok".to_string()))
                } else {
                    Ok(Protocol::err("ERR Discard without MULTI"))
                }
            }
            Cmd::Unknow => Ok(Protocol::err("unknow cmd")),
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

async fn exec_cmd(
    queued_cmd: &mut Option<Vec<(Cmd, Protocol)>>,
    server: &mut Server,
    is_rep_con: bool,
) -> Result<Protocol, DBError> {
    if queued_cmd.is_some() {
        let mut vec = Vec::new();
        for (cmd, protocol) in queued_cmd.as_ref().unwrap() {
            let res = Box::pin(cmd.run(server, protocol.clone(), is_rep_con, &mut None)).await?;
            vec.push(res);
        }
        *queued_cmd = None;
        Ok(Protocol::Array(vec))
    } else {
        Ok(Protocol::err("ERR EXEC without MULTI"))
    }
}

async fn incr_cmd(server: &mut Server, key: &String) -> Result<Protocol, DBError> {
    let mut storage = server.storage.lock().await;
    let v = storage.get(key);
    // return 1 if key is missing
    let v = v.map_or("1".to_string(), |v| v);

    if let Ok(x) = v.parse::<u64>() {
        let v = (x + 1).to_string();
        storage.set(key.clone(), v.clone());
        Ok(Protocol::SimpleString(v))
    } else {
        Ok(Protocol::err("ERR value is not an integer or out of range"))
    }
}

fn config_get_cmd(name: &String, server: &mut Server) -> Result<Protocol, DBError> {
    match name.as_str() {
        "dir" => Ok(Protocol::Array(vec![
            Protocol::BulkString(name.clone()),
            Protocol::BulkString(server.option.dir.clone()),
        ])),
        "dbfilename" => Ok(Protocol::Array(vec![
            Protocol::BulkString(name.clone()),
            Protocol::BulkString(server.option.db_file_name.clone()),
        ])),
        _ => Err(DBError(format!("unsupported config {:?}", name))),
    }
}

async fn keys_cmd(server: &mut Server) -> Result<Protocol, DBError> {
    let keys = { server.storage.lock().await.keys() };
    Ok(Protocol::Array(
        keys.into_iter().map(Protocol::BulkString).collect(),
    ))
}

fn info_cmd(section: &Option<String>, server: &mut Server) -> Result<Protocol, DBError> {
    match section {
        Some(s) => match s.as_str() {
            "replication" => Ok(Protocol::BulkString(format!(
                "role:{}\nmaster_replid:{}\nmaster_repl_offset:{}\n",
                server.option.replication.role,
                server.option.replication.master_replid,
                server.option.replication.master_repl_offset
            ))),
            _ => Err(DBError(format!("unsupported section {:?}", s))),
        },
        None => Ok(Protocol::BulkString("default".to_string())),
    }
}

async fn xread_cmd(
    starts: &[String],
    server: &mut Server,
    stream_keys: &[String],
    block_millis: &Option<u64>,
) -> Result<Protocol, DBError> {
    if let Some(t) = block_millis {
        if t > &0 {
            tokio::time::sleep(Duration::from_millis(*t)).await;
        } else {
            let (sender, mut receiver) = mpsc::channel(4);
            {
                let mut blocker = server.stream_reader_blocker.lock().await;
                blocker.push(sender.clone());
            }
            while let Some(_) = receiver.recv().await {
                println!("get new xadd cmd, release block");
                // break;
            }
        }
    }
    let streams = server.streams.lock().await;
    let mut ret = Vec::new();
    for (i, stream_key) in stream_keys.iter().enumerate() {
        let stream = streams.get(stream_key);
        if let Some(s) = stream {
            let (offset_id, mut offset_seq, _) = split_offset(starts[i].as_str());
            offset_seq += 1;
            let start = format!("{}-{}", offset_id, offset_seq);
            let end = format!("{}-{}", u64::MAX - 1, 0);

            // query stream range
            let range = s.range::<String, _>((Bound::Included(&start), Bound::Included(&end)));
            let mut array = Vec::new();
            for (k, v) in range {
                array.push(Protocol::BulkString(k.clone()));
                array.push(Protocol::from_vec(
                    v.iter()
                        .flat_map(|(a, b)| vec![a.as_str(), b.as_str()])
                        .collect(),
                ))
            }
            ret.push(Protocol::BulkString(stream_key.clone()));
            ret.push(Protocol::Array(array));
        }
    }
    Ok(Protocol::Array(ret))
}

fn replconf_cmd(sub_cmd: &str, server: &mut Server) -> Result<Protocol, DBError> {
    match sub_cmd {
        "getack" => Ok(Protocol::from_vec(vec![
            "REPLCONF",
            "ACK",
            server
                .offset
                .load(std::sync::atomic::Ordering::Relaxed)
                .to_string()
                .as_str(),
        ])),
        _ => Ok(Protocol::SimpleString("OK".to_string())),
    }
}

async fn xrange_cmd(
    server: &mut Server,
    stream_key: &String,
    start: &String,
    end: &String,
) -> Result<Protocol, DBError> {
    let streams = server.streams.lock().await;
    let stream = streams.get(stream_key);
    Ok(stream.map_or(Protocol::none(), |s| {
        // support query with '-'
        let start = if start == "-" {
            "0".to_string()
        } else {
            start.clone()
        };

        // support query with '+'
        let end = if end == "+" {
            u64::MAX.to_string()
        } else {
            end.clone()
        };

        // query stream range
        let range = s.range::<String, _>((Bound::Included(&start), Bound::Included(&end)));
        let mut array = Vec::new();
        for (k, v) in range {
            array.push(Protocol::BulkString(k.clone()));
            array.push(Protocol::from_vec(
                v.iter()
                    .flat_map(|(a, b)| vec![a.as_str(), b.as_str()])
                    .collect(),
            ))
        }
        println!("after xrange: {:?}", array);
        Protocol::Array(array)
    }))
}

async fn xadd_cmd(
    offset: &str,
    server: &mut Server,
    stream_key: &str,
    kvps: &Vec<(String, String)>,
    protocol: Protocol,
    is_rep_con: bool,
) -> Result<Protocol, DBError> {
    let mut offset = offset.to_string();
    if offset == "*" {
        offset = format!("{}-*", now_in_millis() as u64);
    }
    let (offset_id, mut offset_seq, has_wildcard) = split_offset(offset.as_str());
    if offset_id == 0 && offset_seq == 0 && !has_wildcard {
        return Ok(Protocol::err(
            "ERR The ID specified in XADD must be greater than 0-0",
        ));
    }
    {
        let mut streams = server.streams.lock().await;
        let stream = streams
            .entry(stream_key.to_string())
            .or_insert_with(BTreeMap::new);

        if let Some((last_offset, _)) = stream.last_key_value() {
            let (last_offset_id, last_offset_seq, _) = split_offset(last_offset.as_str());
            if last_offset_id > offset_id
                || (last_offset_id == offset_id && last_offset_seq >= offset_seq && !has_wildcard)
            {
                return Ok(Protocol::err("ERR The ID specified in XADD is equal or smaller than the target stream top item"));
            }

            if last_offset_id == offset_id && last_offset_seq >= offset_seq && has_wildcard {
                offset_seq = last_offset_seq + 1;
            }
        }

        let offset = format!("{}-{}", offset_id, offset_seq);

        let s = stream.entry(offset.clone()).or_insert_with(Vec::new);
        for (key, value) in kvps {
            s.push((key.clone(), value.clone()));
        }
    }
    {
        let mut blocker = server.stream_reader_blocker.lock().await;
        for sender in blocker.iter() {
            sender.send(()).await?;
        }
        blocker.clear();
    }
    resp_and_replicate(
        server,
        Protocol::BulkString(offset.to_string()),
        protocol,
        is_rep_con,
    )
    .await
}

async fn type_cmd(server: &mut Server, k: &String) -> Result<Protocol, DBError> {
    let v = { server.storage.lock().await.get(k) };
    if v.is_some() {
        return Ok(Protocol::SimpleString("string".to_string()));
    }
    let streams = server.streams.lock().await;
    let v = streams.get(k);
    Ok(v.map_or(Protocol::none(), |_| {
        Protocol::SimpleString("stream".to_string())
    }))
}

fn psync_cmd(server: &mut Server) -> Result<Protocol, DBError> {
    if server.is_master() {
        Ok(Protocol::SimpleString(format!(
            "FULLRESYNC {} 0",
            server.option.replication.master_replid
        )))
    } else {
        Ok(Protocol::psync_on_slave_err())
    }
}

async fn del_cmd(
    server: &mut Server,
    k: &str,
    protocol: Protocol,
    is_rep_con: bool,
) -> Result<Protocol, DBError> {
    // offset
    let _ = {
        let mut s = server.storage.lock().await;
        s.del(k.to_string());
        server
            .offset
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    };
    resp_and_replicate(server, Protocol::ok(), protocol, is_rep_con).await
}

async fn set_ex_cmd(
    server: &mut Server,
    k: &str,
    v: &str,
    x: &u128,
    protocol: Protocol,
    is_rep_con: bool,
) -> Result<Protocol, DBError> {
    // offset
    let _ = {
        let mut s = server.storage.lock().await;
        s.setx(k.to_string(), v.to_string(), *x * 1000);
        server
            .offset
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    };
    resp_and_replicate(server, Protocol::ok(), protocol, is_rep_con).await
}

async fn set_px_cmd(
    server: &mut Server,
    k: &str,
    v: &str,
    x: &u128,
    protocol: Protocol,
    is_rep_con: bool,
) -> Result<Protocol, DBError> {
    // offset
    let _ = {
        let mut s = server.storage.lock().await;
        s.setx(k.to_string(), v.to_string(), *x);
        server
            .offset
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    };
    resp_and_replicate(server, Protocol::ok(), protocol, is_rep_con).await
}

async fn set_cmd(
    server: &mut Server,
    k: &str,
    v: &str,
    protocol: Protocol,
    is_rep_con: bool,
) -> Result<Protocol, DBError> {
    // offset
    let _ = {
        let mut s = server.storage.lock().await;
        s.set(k.to_string(), v.to_string());
        server
            .offset
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1
    };
    resp_and_replicate(server, Protocol::ok(), protocol, is_rep_con).await
}

async fn get_cmd(server: &mut Server, k: &str) -> Result<Protocol, DBError> {
    let v = {
        let mut s = server.storage.lock().await;
        s.get(k)
    };
    Ok(v.map_or(Protocol::Null, Protocol::SimpleString))
}

async fn resp_and_replicate(
    server: &mut Server,
    resp: Protocol,
    replication: Protocol,
    is_rep_con: bool,
) -> Result<Protocol, DBError> {
    if server.is_master() {
        server
            .master_repl_clients
            .lock()
            .await
            .as_mut()
            .unwrap()
            .send_command(replication)
            .await?;
        Ok(resp)
    } else if !is_rep_con {
        Ok(Protocol::write_on_slave_err())
    } else {
        Ok(resp)
    }
}

fn split_offset(offset: &str) -> (u64, u64, bool) {
    let offset_split = offset.split('-').collect::<Vec<_>>();
    let offset_id = offset_split[0].parse::<u64>().expect(&format!(
        "ERR The ID specified in XADD must be a number: {}",
        offset
    ));

    if offset_split.len() == 1 || offset_split[1] == "*" {
        return (offset_id, if offset_id == 0 { 1 } else { 0 }, true);
    }

    let offset_seq = offset_split[1].parse::<u64>().unwrap();
    (offset_id, offset_seq, false)
}
