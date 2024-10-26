use core::str;
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
use crate::replication_client::FollowerReplicationClient;
use crate::replication_client::MasterReplicationClient;
use crate::storage::Storage;

#[derive(Clone)]
pub struct Server {
    pub storage: Arc<Mutex<Storage>>,
    pub option: options::DBOption,
    follower_repl_client: FollowerReplicationClient,
    master_repl_client: MasterReplicationClient,
}

impl Server {
    pub async fn new(option: options::DBOption) -> Self {
        let master_addr = match option.replication.role.as_str() {
            "slave" => Some(
                option
                    .replication
                    .replica_of
                    .clone()
                    .unwrap()
                    .replace(' ', ":"),
            ),
            _ => None,
        };

        let mut server = Server {
            storage: Arc::new(Mutex::new(Storage::new())),
            option: option,
            follower_repl_client: FollowerReplicationClient::new(master_addr).await,
            master_repl_client: MasterReplicationClient::new(),
        };

        server.init().await.unwrap();
        server
    }

    pub async fn init(self: &mut Self) -> Result<(), DBError> {
        if self.option.replication.role == "slave" {
            // follower initialization
            println!("Start as follower");
            self.follower_repl_client.ping_master().await?;
            self.follower_repl_client
                .report_port(self.option.port)
                .await?;
            self.follower_repl_client.report_sync_protocol().await?;
            self.follower_repl_client.start_psync().await?;
        } else {
            // master initialization
            println!("Start as master");
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
                rdb::parse_rdb_file(&file, self)?;
            }
        }

        Ok(())
    }

    pub async fn handle(self: &mut Self, mut stream: tokio::net::TcpStream) -> Result<(), DBError> {
        let mut buf = [0; 512];
        loop {
            if let Ok(len) = stream.read(&mut buf).await {
                if len == 0 {
                    println!("[handle] connection closed");
                    return Ok(());
                }
                let s = str::from_utf8(&buf[..len])?;
                let cmd = Cmd::from(s)?;
                let res = cmd.run(self)?;
                println!("going to send response {}", res.encode());
                stream.write(res.encode().as_bytes()).await?;

                // send a full RDB file to slave
                match cmd {
                    Cmd::Psync(_, _) => {
                        self.master_repl_client.send_rdb_file(&mut stream).await?;
                    }
                    _ => {} // do nothing for other commands
                }
                println!("finish processing");
            } else {
                println!("[handle] going to break");
                break;
            }
        }
        Ok(())
    }
}
