use anyhow::Result;
use core::str;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::cmd::Cmd;

pub async fn handle(mut stream: tokio::net::TcpStream) {
    let mut buf = [0; 512];
    loop {
        if let Ok(len) = stream.read(&mut buf).await {
            if len == 0 {
                println!("[handle] connection closed");
                return;
            }
            let s = str::from_utf8(&buf[..len]).unwrap();
            let cmd = Cmd::from(s).unwrap();
            let res = cmd.run().unwrap();
            println!("going to send response {}", res.encode());
            stream.write_all(res.encode().as_bytes()).await.unwrap();
            println!("finish processing");
        } else {
            println!("[handle] going to break");
            break;
        }
    }
}
