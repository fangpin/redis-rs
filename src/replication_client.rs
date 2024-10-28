use std::{num::ParseIntError, sync::Arc};

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::{mpsc, Mutex},
};

use crate::{error::DBError, protocol::Protocol};

const EMPTY_RDB_FILE_HEX_STRING: &str = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";

#[derive(Clone)]
pub struct FollowerReplicationClient {
    master_addr: Option<String>,
    stream: Arc<Mutex<Option<TcpStream>>>,
}

impl FollowerReplicationClient {
    pub async fn new(addr: Option<String>) -> FollowerReplicationClient {
        let stream = match addr.clone() {
            Some(address) => {
                let connection = TcpStream::connect(address).await.unwrap();
                Some(connection)
            }
            None => None,
        };

        FollowerReplicationClient {
            master_addr: addr,
            stream: Arc::new(Mutex::new(stream)),
        }
    }

    pub async fn ping_master(self: &mut Self) -> Result<(), DBError> {
        let protocol = Protocol::Array(vec![Protocol::BulkString("PING".to_string())]);
        self.stream
            .lock()
            .await
            .as_mut()
            .unwrap()
            .write_all(protocol.encode().as_bytes())
            .await?;

        self.check_resp("PONG").await
    }

    pub async fn report_port(self: &mut Self, port: u16) -> Result<(), DBError> {
        let protocol = Protocol::form_vec(vec![
            "REPLCONF",
            "listening-port",
            port.to_string().as_str(),
        ]);
        self.stream
            .lock()
            .await
            .as_mut()
            .unwrap()
            .write_all(protocol.encode().as_bytes())
            .await?;

        self.check_resp("OK").await
    }

    pub async fn report_sync_protocol(self: &mut Self) -> Result<(), DBError> {
        let p = Protocol::form_vec(vec!["REPLCONF", "capa", "psync2"]);
        self.stream
            .lock()
            .await
            .as_mut()
            .unwrap()
            .write_all(p.encode().as_bytes())
            .await?;
        self.check_resp("OK").await
    }

    pub async fn start_psync(self: &mut Self) -> Result<(), DBError> {
        let p = Protocol::form_vec(vec!["PSYNC", "?", "-1"]);
        self.stream
            .lock()
            .await
            .as_mut()
            .unwrap()
            .write_all(p.encode().as_bytes())
            .await?;
        self.recv_rdb_file().await?;
        Ok(())
    }

    pub async fn recv_rdb_file(self: &mut Self) -> Result<(), DBError> {
        let mut stream = self.stream.lock().await;
        let stream = stream.as_mut().unwrap();
        let mut reader = BufReader::new(stream);

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

        println!("going to recv rdb file");
        loop {
            let mut buf = vec![0u8; 4096];
            let n_bytes = reader.read(&mut buf).await?;
            if n_bytes == 0 {
                println!("Get rdb file end");
                break;
            }
            println!("recv rdb file: {:?}", &buf[..n_bytes]);
        }
        Ok(())
    }

    pub async fn check_resp(self: &mut Self, expected: &str) -> Result<(), DBError> {
        let mut buf = [0; 1024];
        let n_bytes = self
            .stream
            .lock()
            .await
            .as_mut()
            .unwrap()
            .read(&mut buf)
            .await?;
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
pub struct MasterReplicationClient {}

impl MasterReplicationClient {
    pub fn new() -> MasterReplicationClient {
        MasterReplicationClient {}
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

    pub async fn store_command(
        self: &mut Self,
        replication_sender: mpsc::Sender<(Protocol, u64)>,
        protocol: Protocol,
        offset: u64,
    ) -> Result<(), DBError> {
        replication_sender.send((protocol, offset)).await?;
        Ok(())
    }

    pub async fn send_commands(
        self: &mut Self,
        replication_receiver: Arc<Mutex<mpsc::Receiver<(Protocol, u64)>>>,
        stream: &mut TcpStream,
    ) -> Result<(), DBError> {
        let mut receiver = replication_receiver.lock().await;
        while let Some((protocol, offset)) = receiver.recv().await {
            println!(
                "going to send command {:?} with offset {}",
                protocol, offset
            );
            stream.write_all(protocol.encode().as_bytes()).await?;
        }
        Ok(())
    }
}
