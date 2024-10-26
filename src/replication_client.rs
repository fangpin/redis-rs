use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
};

use crate::{error::DBError, protocol::Protocol};

#[derive(Clone)]
pub struct ReplicationClient {
    master_addr: Option<String>,
    stream: Arc<Mutex<Option<TcpStream>>>,
}

impl ReplicationClient {
    pub fn new(addr: Option<String>) -> ReplicationClient {
        ReplicationClient {
            master_addr: addr.clone(),
            stream: Arc::new(Mutex::new(addr.map(|x| TcpStream::connect(x).unwrap()))),
        }
    }

    pub fn ping_master(self: &mut Self) -> Result<(), DBError> {
        let protocol = Protocol::Array(vec![Protocol::BulkString("PING".to_string())]);
        self.stream
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .write_all(protocol.encode().as_bytes())?;

        self.check_resp("PONG")
    }

    pub fn report_port(self: &mut Self, port: u16) -> Result<(), DBError> {
        let protocol = Protocol::form_vec(vec![
            "REPLCONF",
            "listening-port",
            port.to_string().as_str(),
        ]);
        self.stream
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .write_all(protocol.encode().as_bytes())?;

        self.check_resp("OK")
    }

    pub fn report_sync_protocol(self: &mut Self) -> Result<(), DBError> {
        let p = Protocol::form_vec(vec!["REPLCONF", "capa", "psync2"]);
        self.stream
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .write_all(p.encode().as_bytes())?;
        self.check_resp("OK")
    }

    pub fn communicate_offset(self: &mut Self) -> Result<(), DBError> {
        let p = Protocol::form_vec(vec!["PSYNC", "?", "-1"]);
        self.stream
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .write_all(p.encode().as_bytes())?;
        Ok(())
    }

    pub fn check_resp(self: &mut Self, expected: &str) -> Result<(), DBError> {
        let mut buf = [0; 1024];
        let n_bytes = self
            .stream
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .read(&mut buf)?;
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
