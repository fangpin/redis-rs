use std::{num::ParseIntError, sync::Arc};

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::Mutex,
};

use crate::{error::DBError, protocol::Protocol, rdb};

const EMPTY_RDB_FILE_HEX_STRING: &str = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";

pub struct FollowerReplicationClient {
    master_addr: String,
    pub stream: TcpStream,
}

impl FollowerReplicationClient {
    pub async fn new(addr: String) -> FollowerReplicationClient {
        FollowerReplicationClient {
            master_addr: addr.clone(),
            stream: TcpStream::connect(addr).await.unwrap(),
        }
    }

    pub async fn ping_master(self: &mut Self) -> Result<(), DBError> {
        let protocol = Protocol::Array(vec![Protocol::BulkString("PING".to_string())]);
        self.stream.write_all(protocol.encode().as_bytes()).await?;

        self.check_resp("PONG").await
    }

    pub async fn report_port(self: &mut Self, port: u16) -> Result<(), DBError> {
        let protocol = Protocol::form_vec(vec![
            "REPLCONF",
            "listening-port",
            port.to_string().as_str(),
        ]);
        self.stream.write_all(protocol.encode().as_bytes()).await?;

        self.check_resp("OK").await
    }

    pub async fn report_sync_protocol(self: &mut Self) -> Result<(), DBError> {
        let p = Protocol::form_vec(vec!["REPLCONF", "capa", "psync2"]);
        self.stream.write_all(p.encode().as_bytes()).await?;
        self.check_resp("OK").await
    }

    pub async fn start_psync(self: &mut Self) -> Result<(), DBError> {
        let p = Protocol::form_vec(vec!["PSYNC", "?", "-1"]);
        self.stream.write_all(p.encode().as_bytes()).await?;
        self.recv_rdb_file().await?;
        Ok(())
    }

    pub async fn recv_rdb_file(self: &mut Self) -> Result<(), DBError> {
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

        // receive rdb file content
        let mut rdb_content = Vec::new();
        reader.read_until(rdb::EOF, &mut rdb_content).await?;
        let mut crc_buf = [0; 8];
        let _crc = reader.read_exact(&mut crc_buf).await?;
        rdb_content.extend_from_slice(&crc_buf);
        println!("recv rdb file: {:?}", &rdb_content);
        Ok(())
    }

    pub async fn check_resp(self: &mut Self, expected: &str) -> Result<(), DBError> {
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

    pub async fn send_rdb_file(self: &mut Self, stream: &mut TcpStream) -> Result<(), DBError> {
        let empty_rdb_file_bytes = (0..EMPTY_RDB_FILE_HEX_STRING.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&EMPTY_RDB_FILE_HEX_STRING[i..i + 2], 16))
            .collect::<Result<Vec<u8>, ParseIntError>>()?;

        println!("going to send rdb file");
        stream.write_all("$".as_bytes()).await?;
        stream
            .write_all(empty_rdb_file_bytes.len().to_string().as_bytes())
            .await?;
        stream.write_all("\r\n".as_bytes()).await?;
        stream.write_all(&empty_rdb_file_bytes).await?;
        Ok(())
    }

    pub async fn add_stream(self: &mut Self, stream: TcpStream) -> Result<(), DBError> {
        let mut streams = self.streams.lock().await;
        streams.push(stream);
        Ok(())
    }

    pub async fn send_command(self: &mut Self, protocol: Protocol) -> Result<(), DBError> {
        let mut streams = self.streams.lock().await;
        for stream in streams.iter_mut() {
            stream.write_all(protocol.encode().as_bytes()).await?;
        }
        Ok(())
    }
}
