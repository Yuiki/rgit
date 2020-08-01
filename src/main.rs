use flate2::write::ZlibEncoder;
use flate2::Compression;
use hex;
use sha1::{Digest, Sha1};
use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

const DIR_ROOT: &'static str = ".rgit";

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

    add_entry_to_index(file_path, hashed.to_vec())?;

    Ok(())
}

fn add_entry_to_index(file_path: &String, sha1_obj: Vec<u8>) -> std::io::Result<()> {
    let path = DIR_ROOT.to_string() + "/index";
    let path = Path::new(&path);
    if path.exists() {
        let mut file = File::open(path)?;
    } else {
        let mut file = File::create(path)?;

        let mut buf = vec![];
        write!(buf, "{}", "DIRC")?;
        let ver: u32 = 0x0002;
        buf.write_all(&ver.to_be_bytes())?;

        let entry_size: u32 = 1;
        buf.write_all(&entry_size.to_be_bytes())?;

        let meta = fs::metadata(file_path)?;
        let ctime: u32 = meta.ctime() as u32;
        let ctime_nsec: u32 = meta.ctime_nsec() as u32;
        let mtime: u32 = meta.mtime() as u32;
        let mtime_nsec: u32 = meta.mtime_nsec() as u32;
        let dev: u32 = meta.dev() as u32;
        let ino: u32 = meta.ino() as u32;
        let mode: u32 = meta.mode();
        let uid: u32 = meta.uid();
        let gid: u32 = meta.gid();
        let size: u32 = meta.size() as u32;

        buf.write_all(&ctime.to_be_bytes())?;
        buf.write_all(&ctime_nsec.to_be_bytes())?;
        buf.write_all(&mtime.to_be_bytes())?;
        buf.write_all(&mtime_nsec.to_be_bytes())?;
        buf.write_all(&dev.to_be_bytes())?;
        buf.write_all(&ino.to_be_bytes())?;
        buf.write_all(&mode.to_be_bytes())?;
        buf.write_all(&uid.to_be_bytes())?;
        buf.write_all(&gid.to_be_bytes())?;
        buf.write_all(&size.to_be_bytes())?;

        buf.write_all(&sha1_obj)?;

        let name = Path::new(file_path).file_name().unwrap();
        let name_len: u16 = name.len() as u16;
        buf.write_all(&name_len.to_be_bytes())?;
        write!(buf, "{}", name.to_str().unwrap())?;

        // null padding
        let pad_len = buf.len() % 8;
        for _ in 0..pad_len {
            buf.push(0);
        }

        let mut hasher = Sha1::new();
        hasher.update(&buf);
        let hashed = hasher.finalize();
        buf.write_all(&hashed)?;

        file.write_all(&buf)?;
    }

    Ok(())
}
