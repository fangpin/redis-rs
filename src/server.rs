use core::str;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::cmd::Cmd;
use crate::error::DBError;
use crate::options;
use crate::protocol::Protocol;
use crate::rdb;
use crate::replication_client::FollowerReplicationClient;
use crate::replication_client::MasterReplicationClient;
use crate::storage::Storage;

#[derive(Clone)]
pub struct Server {
    pub storage: Arc<Mutex<Storage>>,
    pub option: options::DBOption,
    pub offset: Arc<AtomicU64>,
    pub master_repl_client: MasterReplicationClient,
    follower_repl_client: FollowerReplicationClient,
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
            offset: Arc::new(AtomicU64::new(0)),
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
                rdb::parse_rdb_file(&file, self).await?;
            }
        }

        Ok(())
    }

    pub async fn handle(
        self: &mut Self,
        mut stream: tokio::net::TcpStream,
        replication_sender: mpsc::Sender<(Protocol, u64)>,
        replication_receiver: Arc<Mutex<mpsc::Receiver<(Protocol, u64)>>>,
    ) -> Result<(), DBError> {
        let mut buf = [0; 512];
        loop {
            if let Ok(len) = stream.read(&mut buf).await {
                if len == 0 {
                    println!("[handle] connection closed");
                    return Ok(());
                }
                let s = str::from_utf8(&buf[..len])?;
                let (cmd, protocol) = Cmd::from(s)?;
                let res = cmd.run(self, protocol, replication_sender.clone()).await?;
                println!("going to send response {}", res.encode());
                stream.write(res.encode().as_bytes()).await?;

                // send a full RDB file to slave
                match cmd {
                    Cmd::Psync(_, _) => {
                        self.master_repl_client.send_rdb_file(&mut stream).await?;

                        self.master_repl_client
                            .send_commands(replication_receiver.clone(), &mut stream)
                            .await?;
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
