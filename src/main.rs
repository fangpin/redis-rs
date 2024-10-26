// #![allow(unused_imports)]

use redis_rs::{options::ReplicationOption, server};

use clap::Parser;
use tokio::net::TcpListener;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The directory of Redis DB file
    #[arg(long)]
    dir: String,

    /// The name of the Redis DB file
    #[arg(long)]
    dbfilename: String,

    /// The port of the Redis server, default is 6379 if not specified
    #[arg(long)]
    port: Option<u16>,

    /// The address of the master Redis server, if the server is a replica. None if the server is a master.
    #[arg(long)]
    replicaof: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let port = args.port.unwrap_or(6379);
    println!("will listen on port: {}", port);
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();

    let option = redis_rs::options::DBOption {
        dir: args.dir,
        db_file_name: args.dbfilename,
        port: port,
        replication: ReplicationOption {
            role: if let Some(_) = args.replicaof {
                "slave".to_string()
            } else {
                "master".to_string()
            },
            master_replid: "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeea".to_string(), // should be a random string but hard code for now
            master_repl_offset: 0,
            replica_of: args.replicaof,
        },
    };

    let server = server::Server::new(option).await;

    loop {
        let stream = listener.accept().await;
        match stream {
            Ok((stream, _)) => {
                println!("accepted new connection");

                let mut sc = server.clone();
                tokio::spawn(async move {
                    if let Err(e) = sc.handle(stream).await {
                        println!("error: {:?}, will close the connection. Bye", e);
                    }
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
