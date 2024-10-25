// #![allow(unused_imports)]

use redis_rs::server;

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
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let option = redis_rs::options::DBOption {
        dir: args.dir,
        db_file_name: args.dbfilename,
        redis_version: String::new(),
        place_holder: String::new(),
    };

    let port = args.port.unwrap_or(6379);
    println!("will listen on port: {}", port);

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();
    let server = server::Server::new(option);

    loop {
        let stream = listener.accept().await;
        match stream {
            Ok((stream, _)) => {
                println!("accepted new connection");

                let mut sc = server.clone();
                tokio::spawn(async move {
                    sc.handle(stream).await;
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
