use core::str;
use std::fs;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

use crate::cmd::Cmd;
use crate::error::DBError;
use crate::options;
use crate::rdb;
use crate::storage::Storage;

#[derive(Clone)]
pub struct Server {
    pub storage: Arc<Mutex<Storage>>,
    pub option: options::DBOption,
}

impl Server {
    pub fn new(option: options::DBOption) -> Self {
        let mut server = Server {
            storage: Arc::new(Mutex::new(Storage::new())),
            option: option,
        };

        server.init().unwrap();
        server
    }

    pub fn init(self: &mut Self) -> Result<(), DBError> {
        let db_file_path =
            PathBuf::from(self.option.dir.clone()).join(self.option.db_file_name.clone());
        println!("will open db file path: {}", db_file_path.display());

        // create empty db file if not exits
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(db_file_path.clone())?;

        if file.metadata()?.len() != 0 {
            rdb::parse_rdb_file(&file, self).unwrap();
        }

        Ok(())
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
