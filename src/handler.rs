use core::str;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use anyhow::Result;

use crate::cmd::Cmd;

pub async fn handle(mut stream: tokio::net::TcpStream) -> Result<()> {
    let mut buf = [0; 512];
    loop {
        let len = stream.read(&mut buf).await.unwrap();
        if len == 0 {
            break;
        }
        let s = str::from_utf8(&buf[0..len])?;
        let cmd = Cmd::from(s)?;
        let res = cmd.run()?;
        stream.write(res.encode().as_bytes()).await.unwrap();
    }
    return Ok(())
}