use core::str;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use crate::cmd::Cmd;
use crate::error::DBError;
use crate::options;
use crate::rdb;
use crate::replication_client::FollowerReplicationClient;
use crate::replication_client::MasterReplicationClient;
use crate::storage::Storage;

type Stream = BTreeMap<String, Vec<(String, String)>>;

#[derive(Clone)]
pub struct Server {
    pub storage: Arc<Mutex<Storage>>,
    pub streams: Arc<Mutex<HashMap<String, Stream>>>,
    pub option: options::DBOption,
    pub offset: Arc<AtomicU64>,
    pub master_repl_clients: Arc<Mutex<Option<MasterReplicationClient>>>,
    pub stream_reader_blocker: Arc<Mutex<Vec<Sender<()>>>>,
    master_addr: Option<String>,
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

        let is_master = option.replication.role == "master";

        let mut server = Server {
            storage: Arc::new(Mutex::new(Storage::new())),
            streams: Arc::new(Mutex::new(HashMap::new())),
            option,
            master_repl_clients: if is_master {
                Arc::new(Mutex::new(Some(MasterReplicationClient::new())))
            } else {
                Arc::new(Mutex::new(None))
            },
            offset: Arc::new(AtomicU64::new(0)),
            stream_reader_blocker: Arc::new(Mutex::new(Vec::new())),
            master_addr,
        };

        server.init().await.unwrap();
        server
    }

    pub async fn init(&mut self) -> Result<(), DBError> {
        // master initialization
        if self.is_master() {
            println!("Start as master\n");
            let db_file_path =
                PathBuf::from(self.option.dir.clone()).join(self.option.db_file_name.clone());
            println!("will open db file path: {}", db_file_path.display());

            // create empty db file if not exits
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(db_file_path.clone())
                .await?;

            if file.metadata().await?.len() != 0 {
                rdb::parse_rdb_file(&mut file, self).await?;
            }
        }
        Ok(())
    }

    pub async fn get_follower_repl_client(&mut self) -> Option<FollowerReplicationClient> {
        if self.is_slave() {
            Some(FollowerReplicationClient::new(self.master_addr.clone().unwrap()).await)
        } else {
            None
        }
    }

    pub async fn handle(
        &mut self,
        mut stream: tokio::net::TcpStream,
        is_rep_conn: bool,
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
                println!("got command: {:?}, protocol: {:?}", cmd, protocol);

                let res = cmd.run(self, protocol, is_rep_conn).await?;

                // only send response to normal client, do not send response to replication client
                if !is_rep_conn {
                    println!("going to send response {}", res.encode());
                    _ = stream.write(res.encode().as_bytes()).await?;
                }

                // send a full RDB file to slave
                if self.is_master() {
                    if let Cmd::Psync = cmd {
                        let mut master_rep_client = self.master_repl_clients.lock().await;
                        let master_rep_client = master_rep_client.as_mut().unwrap();
                        master_rep_client.send_rdb_file(&mut stream).await?;
                        master_rep_client.add_stream(stream).await?;
                        break;
                    }
                }
            } else {
                println!("[handle] going to break");
                break;
            }
        }
        Ok(())
    }

    pub fn is_slave(&self) -> bool {
        self.option.replication.role == "slave"
    }

    pub fn is_master(&self) -> bool {
        !self.is_slave()
    }
}
