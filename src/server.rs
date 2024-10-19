use core::str;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

use crate::cmd::Cmd;
use crate::options;
use crate::storage::Storage;

#[derive(Clone)]
pub struct Server {
    pub storage: Arc<Mutex<Storage>>,
    pub option: options::DBOption,
}

impl Server {
    pub fn new(option: options::DBOption) -> Self {
        Server {
            storage: Arc::new(Mutex::new(Storage::new())),
            option: option,
        }
    }

    pub async fn handle(self: &mut Self, mut stream: tokio::net::TcpStream) {
        let mut buf = [0; 512];
        loop {
            if let Ok(len) = stream.read(&mut buf).await {
                if len == 0 {
                    println!("[handle] connection closed");
                    return;
                }
                let s = str::from_utf8(&buf[..len]).unwrap();
                let cmd = Cmd::from(s).unwrap();
                let res = cmd.run(self).unwrap();
                println!("going to send response {}", res.encode());
                stream.write_all(res.encode().as_bytes()).await.unwrap();
                println!("finish processing");
            } else {
                println!("[handle] going to break");
                break;
            }
        }
    }
}
