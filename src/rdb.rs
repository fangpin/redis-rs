// parse Redis RDB file format: https://rdb.fnordig.de/file_format.html

use tokio::{
    fs,
    io::{AsyncRead, AsyncReadExt, BufReader},
};

use crate::{error::DBError, server::Server};

use futures::pin_mut;

enum StringEncoding {
    Raw,
    I8,
    I16,
    I32,
    LZF,
}

// RDB file format.
const MAGIC: &[u8; 5] = b"REDIS";
const META: u8 = 0xFA;
const DB_SELECT: u8 = 0xFE;
const TABLE_SIZE_INFO: u8 = 0xFB;
pub const EOF: u8 = 0xFF;

pub async fn parse_rdb<R: AsyncRead + Unpin>(
    reader: &mut R,
    server: &mut Server,
) -> Result<(), DBError> {
    let mut storage = server.storage.lock().await;
    parse_magic(reader).await?;
    let _version = parse_version(reader).await?;
    pin_mut!(reader);
    loop {
        let op = reader.read_u8().await?;
        match op {
            META => {
                let _ = parse_aux(&mut *reader).await?;
                let _ = parse_aux(&mut *reader).await?;
                // just ignore the aux info for now
            }
            DB_SELECT => {
                let (_, _) = parse_len(&mut *reader).await?;
                // just ignore the db index for now
            }
            TABLE_SIZE_INFO => {
                let size_no_expire = parse_len(&mut *reader).await?.0;
                let size_expire = parse_len(&mut *reader).await?.0;
                for _ in 0..size_no_expire {
                    let (k, v) = parse_no_expire_entry(&mut *reader).await?;
                    storage.set(k, v);
                }
                for _ in 0..size_expire {
                    let (k, v, expire_timestamp) = parse_expire_entry(&mut *reader).await?;
                    storage.setx(k, v, expire_timestamp);
                }
            }
            EOF => {
                // not verify crc for now
                let _crc = reader.read_u64().await?;
                break;
            }
            _ => return Err(DBError(format!("unexpected op: {}", op))),
        }
    }
    Ok(())
}

pub async fn parse_rdb_file(f: &mut fs::File, server: &mut Server) -> Result<(), DBError> {
    let mut reader = BufReader::new(f);
    parse_rdb(&mut reader, server).await
}

async fn parse_no_expire_entry<R: AsyncRead + Unpin>(
    input: &mut R,
) -> Result<(String, String), DBError> {
    let b = input.read_u8().await?;
    if b != 0 {
        return Err(DBError(format!("unexpected key type: {}", b)));
    }
    let k = parse_aux(input).await?;
    let v = parse_aux(input).await?;
    Ok((k, v))
}

async fn parse_expire_entry<R: AsyncRead + Unpin>(
    input: &mut R,
) -> Result<(String, String, u128), DBError> {
    let b = input.read_u8().await?;
    match b {
        0xFC => {
            // expire in milliseconds
            let expire_stamp = input.read_u64_le().await?;
            let (k, v) = parse_no_expire_entry(input).await?;
            Ok((k, v, expire_stamp as u128))
        }
        0xFD => {
            // expire in seconds
            let expire_timestamp = input.read_u32_le().await?;
            let (k, v) = parse_no_expire_entry(input).await?;
            Ok((k, v, (expire_timestamp * 1000) as u128))
        }
        _ => return Err(DBError(format!("unexpected expire type: {}", b))),
    }
}

async fn parse_magic<R: AsyncRead + Unpin>(input: &mut R) -> Result<(), DBError> {
    let mut magic = [0; 5];
    let size_read = input.read(&mut magic).await?;
    if size_read != 5 {
        Err(DBError("expected 5 chars for magic number".to_string()))
    } else if magic.as_slice() == MAGIC {
        Ok(())
    } else {
        Err(DBError(format!(
            "expected magic string {:?}, but got: {:?}",
            MAGIC, magic
        )))
    }
}

async fn parse_version<R: AsyncRead + Unpin>(input: &mut R) -> Result<[u8; 4], DBError> {
    let mut version = [0; 4];
    let size_read = input.read(&mut version).await?;
    if size_read != 4 {
        Err(DBError("expected 4 chars for redis version".to_string()))
    } else {
        Ok(version)
    }
}

async fn parse_aux<R: AsyncRead + Unpin>(input: &mut R) -> Result<String, DBError> {
    let (len, encoding) = parse_len(input).await?;
    let s = parse_string(input, len, encoding).await?;
    Ok(s)
}

async fn parse_len<R: AsyncRead + Unpin>(input: &mut R) -> Result<(u32, StringEncoding), DBError> {
    let first = input.read_u8().await?;
    match first & 0xC0 {
        0x00 => {
            // The size is the remaining 6 bits of the byte.
            Ok((first as u32, StringEncoding::Raw))
        }
        0x04 => {
            // The size is the next 14 bits of the byte.
            let second = input.read_u8().await?;
            Ok((
                (((first & 0x3F) as u32) << 8 | second as u32) as u32,
                StringEncoding::Raw,
            ))
        }
        0x80 => {
            //Ignore the remaining 6 bits of the first byte.  The size is the next 4 bytes, in big-endian
            let second = input.read_u32().await?;
            Ok((second, StringEncoding::Raw))
        }
        0xC0 => {
            // The remaining 6 bits specify a type of string encoding.
            match first {
                0xC0 => Ok((1, StringEncoding::I8)),
                0xC1 => Ok((2, StringEncoding::I16)),
                0xC2 => Ok((4, StringEncoding::I32)),
                0xC3 => Ok((0, StringEncoding::LZF)), // not supported yet
                _ => Err(DBError(format!("unexpected string encoding: {}", first))),
            }
        }
        _ => Err(DBError(format!("unexpected len prefix: {}", first))),
    }
}

async fn parse_string<R: AsyncRead + Unpin>(
    input: &mut R,
    len: u32,
    encoding: StringEncoding,
) -> Result<String, DBError> {
    match encoding {
        StringEncoding::Raw => {
            let mut s = vec![0; len as usize];
            input.read_exact(&mut s).await?;
            Ok(String::from_utf8(s).unwrap())
        }
        StringEncoding::I8 => {
            let b = input.read_u8().await?;
            Ok(b.to_string())
        }
        StringEncoding::I16 => {
            let b = input.read_u16_le().await?;
            Ok(b.to_string())
        }
        StringEncoding::I32 => {
            let b = input.read_u32_le().await?;
            Ok(b.to_string())
        }
        StringEncoding::LZF => {
            // not supported yet
            Err(DBError("LZF encoding not supported yet".to_string()))
        }
    }
}
