#![allow(unused_imports)]

use redis_starter_rust::handler;

use std::thread::spawn;

use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() {
    println!("Logs from your program will appear here!");
    
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    
    loop {
        let stream = listener.accept().await;
        match stream {
            Ok((mut stream, _)) => {
                println!("accepted new connection");

                handler::handle(stream).await.unwrap();

            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
