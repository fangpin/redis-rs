use std::{num::ParseIntError, sync::Arc};

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::Mutex,
};

use crate::{error::DBError, protocol::Protocol, rdb, server::Server};

const EMPTY_RDB_FILE_HEX_STRING: &str = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";

pub struct FollowerReplicationClient {
    pub stream: TcpStream,
}

impl FollowerReplicationClient {
    pub async fn new(addr: String) -> FollowerReplicationClient {
        FollowerReplicationClient {
            stream: TcpStream::connect(addr).await.unwrap(),
        }
    }

    pub async fn ping_master(self: &mut Self) -> Result<(), DBError> {
        let protocol = Protocol::Array(vec![Protocol::BulkString("PING".to_string())]);
        self.stream.write_all(protocol.encode().as_bytes()).await?;

        self.check_resp("PONG").await
    }

    pub async fn report_port(self: &mut Self, port: u16) -> Result<(), DBError> {
        let protocol = Protocol::from_vec(vec![
            "REPLCONF",
            "listening-port",
            port.to_string().as_str(),
        ]);
        self.stream.write_all(protocol.encode().as_bytes()).await?;

        self.check_resp("OK").await
    }

    pub async fn report_sync_protocol(self: &mut Self) -> Result<(), DBError> {
        let p = Protocol::from_vec(vec!["REPLCONF", "capa", "psync2"]);
        self.stream.write_all(p.encode().as_bytes()).await?;
        self.check_resp("OK").await
    }

    pub async fn start_psync(self: &mut Self, server: &mut Server) -> Result<(), DBError> {
        let p = Protocol::from_vec(vec!["PSYNC", "?", "-1"]);
        self.stream.write_all(p.encode().as_bytes()).await?;
        self.recv_rdb_file(server).await?;
        Ok(())
    }

    pub async fn recv_rdb_file(self: &mut Self, server: &mut Server) -> Result<(), DBError> {
        let mut reader = BufReader::new(&mut self.stream);

        let mut buf = Vec::new();
        let _ = reader.read_until(b'\n', &mut buf).await?;
        buf.pop();
        buf.pop();

        let replication_info = String::from_utf8(buf)?;
        let replication_info = replication_info
            .split_whitespace()
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        if replication_info.len() != 3 {
            return Err(DBError(format!(
                "expect 3 args but found {:?}",
                replication_info
            )));
        }
        println!(
            "Get replication info: {:?} {:?} {:?}",
            replication_info[0], replication_info[1], replication_info[2]
        );

        let c = reader.read_u8().await?;
        if c != b'$' {
            return Err(DBError(format!("expect $ but found {}", c)));
        }
        let mut buf = Vec::new();
        reader.read_until(b'\n', &mut buf).await?;
        buf.pop();
        buf.pop();
        let rdb_file_len = String::from_utf8(buf)?.parse::<usize>()?;
        println!("rdb file len: {}", rdb_file_len);

        // receive rdb file content
        rdb::parse_rdb(&mut reader, server).await?;
        Ok(())
    }

    pub async fn check_resp(&mut self, expected: &str) -> Result<(), DBError> {
        let mut buf = [0; 1024];
        let n_bytes = self.stream.read(&mut buf).await?;
        println!(
            "check resp: recv {:?}",
            String::from_utf8(buf[..n_bytes].to_vec()).unwrap()
        );
        let expect = Protocol::SimpleString(expected.to_string()).encode();
        if expect.as_bytes() != &buf[..n_bytes] {
            return Err(DBError(format!(
                "expect response {:?} but found {:?}",
                expect,
                &buf[..n_bytes]
            )));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct MasterReplicationClient {
    pub streams: Arc<Mutex<Vec<TcpStream>>>,
}

impl MasterReplicationClient {
    pub fn new() -> MasterReplicationClient {
        MasterReplicationClient {
            streams: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn send_rdb_file(&mut self, stream: &mut TcpStream) -> Result<(), DBError> {
        let empty_rdb_file_bytes = (0..EMPTY_RDB_FILE_HEX_STRING.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&EMPTY_RDB_FILE_HEX_STRING[i..i + 2], 16))
            .collect::<Result<Vec<u8>, ParseIntError>>()?;

        println!("going to send rdb file");
        _ = stream.write("$".as_bytes()).await?;
        _ = stream
            .write(empty_rdb_file_bytes.len().to_string().as_bytes())
            .await?;
        _ = stream.write_all("\r\n".as_bytes()).await?;
        _ = stream.write_all(&empty_rdb_file_bytes).await?;
        Ok(())
    }

    pub async fn add_stream(&mut self, stream: TcpStream) -> Result<(), DBError> {
        let mut streams = self.streams.lock().await;
        streams.push(stream);
        Ok(())
    }

    pub async fn send_command(&mut self, protocol: Protocol) -> Result<(), DBError> {
        let mut streams = self.streams.lock().await;
        for stream in streams.iter_mut() {
            stream.write_all(protocol.encode().as_bytes()).await?;
        }
        Ok(())
    }
}
