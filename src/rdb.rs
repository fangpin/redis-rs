// parse Redis RDB file format: https://rdb.fnordig.de/file_format.html

use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Lines},
    iter::Peekable,
};

use crate::{options::DBOption, server::Server};

// RDB file format.
const MetaDataStart: u8 = 0xFA;
const DBSectionStart: u8 = 0xFE;

pub fn parse_db_file(f: &fs::File, server: &mut Server) -> Result<(), String> {
    let reader = BufReader::new(f);
    let mut lines = reader.lines().peekable();
    let mut storage = server.storage.write().unwrap();
    parse_header(&mut lines, &mut server.option)?;
    parse_metadata(&mut lines, &mut server.option)?;
    Ok(())
}

fn parse_header(
    lines: &mut Peekable<Lines<BufReader<&File>>>,
    option: &mut DBOption,
) -> Result<(), String> {
    if let Some(line) = lines.next() {
        let line = line?;
        let bytes = line.as_bytes();
        if bytes.starts_with(b"REDIS") {
            option.redis_version = line;
            Ok(())
        } else {
            Err(format!("unexpected header {:?}", line))
        }
    } else {
        Err(format!("unexpected end of file"))
    }
}

fn parse_metadata(
    lines: &mut Peekable<Lines<BufReader<&File>>>,
    option: &mut DBOption,
) -> Result<(), String> {
    if let Some(line) = lines.next() {
        let line = line?;
        let metadata_start = line.as_bytes()[0];
        if MetaDataStart == metadata_start {
            if let Some(next_line) = lines.peek() {
                let next_line = next_line.as_ref().unwrap();
                if next_line.as_bytes()[0] == DBSectionStart {
                    // end of meta data section
                    return Ok(());
                } else {
                    let k = lines
                        .next()
                        .map(|x| x.unwrap())
                        .ok_or("reading line error")?;
                    let v = lines.next().map(|x| x.unwrap());
                }
            } else {
                return Err(format!("unexpected header {:?}", line));
            }
            Ok(())
        } else {
            Err(format!("expect meta dada start but found {:?}", line))
        }
    } else {
        Err(format!("unexpected end of file"))
    }
}
