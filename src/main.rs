// #![allow(unused_imports)]

use tokio::net::TcpListener;

use redis_rs::{options::ReplicationOption, replication_channel, server};

use clap::Parser;

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
    // parse args
    let args = Args::parse();

    // bind port
    let port = args.port.unwrap_or(6379);
    println!("will listen on port: {}", port);
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();

    // new DB option
    let option = redis_rs::options::DBOption {
        dir: args.dir,
        db_file_name: args.dbfilename,
        port,
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

    // init replication channel
    replication_channel::init();

    // new server
    let server = server::Server::new(option).await;

    //start receive replication cmds for slave
    if server.is_slave() {
        let mut sc = server.clone();
        tokio::spawn(async move {
            loop {
                match sc.clone().follower_repl_client.as_ref() {
                    Some(client) => {
                        let mut stream = client.stream.lock().await;
                        if let Some(mut stream) = stream.as_mut() {
                            if let Err(e) = sc.handle(&mut stream, true).await {
                                println!("error: {:?}, will close the connection. Bye", e);
                            }
                        }
                    }
                    None => println!("No replication client available"),
                }
            }
        });
    }

    // accept new connections
    loop {
        let stream = listener.accept().await;
        match stream {
            Ok((mut stream, _)) => {
                println!("accepted new connection");

                let mut sc = server.clone();
                tokio::spawn(async move {
                    if let Err(e) = sc.handle(&mut stream, false).await {
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
