use flate2::write::ZlibEncoder;
use flate2::Compression;
use hex;
use sha1::{Digest, Sha1};
use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;

const DIR_OBJECTS: &'static str = ".rgit/objects";

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = args[1].as_str();
    match command {
        "init" => {
            init()?;
        }
        "add" => {
            let file_path = &args[2];
            add(file_path)?;
        }
        _ => {}
    };
    Ok(())
}

fn init() -> std::io::Result<()> {
    fs::create_dir_all(DIR_OBJECTS)?;
    Ok(())
}

fn add(file_path: &String) -> std::io::Result<()> {
    let content = fs::read(file_path).unwrap();
    let mut obj: Vec<u8> = vec![];
    obj.extend_from_slice("blob ".as_bytes());
    obj.extend_from_slice(content.len().to_string().as_bytes());
    obj.push(0);
    obj.extend_from_slice(&content);

    let mut hasher = Sha1::new();
    hasher.update(&obj);
    let hashed = hasher.finalize();
    let hashed_hex = hex::encode(&hashed);

    let dir_name = &hashed_hex[0..2];
    let file_name = &hashed_hex[2..];

    let full_dir = DIR_OBJECTS.to_string() + "/" + dir_name;
    fs::create_dir(&full_dir)?;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&obj)?;
    let compressed = encoder.finish()?;

    let mut file = File::create(full_dir.to_string() + "/" + file_name)?;
    file.write_all(&compressed)?;

    Ok(())
}
