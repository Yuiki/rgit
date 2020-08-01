use flate2::write::ZlibEncoder;
use flate2::Compression;
use hex;
use sha1::{Digest, Sha1};
use std::env;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Result;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

const DIR_ROOT: &'static str = ".rgit";

const DIR_OBJECTS: &'static str = "objects";

fn main() -> Result<()> {
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

fn init() -> Result<()> {
    fs::create_dir_all(DIR_ROOT.to_string() + "/" + DIR_OBJECTS)?;
    Ok(())
}

fn add(file_path: &String) -> Result<()> {
    if !Path::new(file_path).exists() {
        return Ok(());
    }

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

    let full_dir = DIR_ROOT.to_string() + "/" + DIR_OBJECTS + "/" + dir_name;
    let dir_path = Path::new(&full_dir);
    if !dir_path.exists() {
        fs::create_dir(&dir_path)?;
    }

    let obj_path_str = full_dir.to_string() + "/" + file_name;
    let obj_path = Path::new(&obj_path_str);
    if !obj_path.exists() {
        let mut file = File::create(obj_path)?;
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&obj)?;
        let compressed = encoder.finish()?;
        file.write_all(&compressed)?;
    }

    let mut sha1_obj = [0u8; 20];
    sha1_obj.copy_from_slice(&hashed);
    add_entry_to_index(&file_path, sha1_obj)?;

    Ok(())
}

fn add_entry_to_index(file_path: &String, sha1_obj: [u8; 20]) -> Result<()> {
    let path = DIR_ROOT.to_string() + "/index";
    let path = Path::new(&path);
    let mut entries = if path.exists() {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut u32_buf = [0u8; 4];

        reader.read(&mut u32_buf)?;
        let _sign = u32::from_be_bytes(u32_buf);

        reader.read(&mut u32_buf)?;
        let _ver = u32::from_be_bytes(u32_buf);

        reader.read(&mut u32_buf)?;
        let entry_count = u32::from_be_bytes(u32_buf);

        let mut entries: Vec<Entry> = vec![];
        for _ in 0..entry_count {
            let entry = Entry::deserialize(&mut reader)?;
            entries.push(entry);
        }
        entries
    } else {
        vec![]
    };

    let found = entries.iter().find(|entry| entry.sha1_obj == sha1_obj);
    if found.is_some() {
        return Ok(());
    }

    let mut file = if path.exists() {
        OpenOptions::new().write(true).open(path)?
    } else {
        File::create(path)?
    };

    let mut buf = vec![];
    write!(buf, "{}", "DIRC")?;
    let ver: u32 = 0x0002;
    buf.write_all(&ver.to_be_bytes())?;

    println!("{}", file_path);
    let meta = fs::metadata(file_path)?;
    let name = Path::new(file_path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    let entry = Entry {
        ctime: meta.ctime() as u32,
        ctime_nsec: meta.ctime_nsec() as u32,
        mtime: meta.mtime() as u32,
        mtime_nsec: meta.mtime_nsec() as u32,
        dev: meta.dev() as u32,
        ino: meta.ino() as u32,
        mode: meta.mode(),
        uid: meta.uid(),
        gid: meta.gid(),
        file_size: meta.size() as u32,
        sha1_obj,
        name,
    };
    entries.push(entry);

    let entry_size = entries.len() as u32;
    buf.write_all(&entry_size.to_be_bytes())?;

    for entry in entries {
        entry.serialize(&mut buf)?;
    }

    let mut hasher = Sha1::new();
    hasher.update(&buf);
    let hashed = hasher.finalize();
    buf.write_all(&hashed)?;

    file.write_all(&buf)?;

    Ok(())
}

struct Entry {
    ctime: u32,
    ctime_nsec: u32,
    mtime: u32,
    mtime_nsec: u32,
    dev: u32,
    ino: u32,
    mode: u32,
    uid: u32,
    gid: u32,
    file_size: u32,
    sha1_obj: [u8; 20],
    name: Vec<u8>,
}

trait Serializable {
    type Item;

    fn serialize(&self, buf: &mut Vec<u8>) -> Result<()>;
    fn deserialize<T: Read>(reader: &mut T) -> Result<Self::Item>;
}

impl Serializable for Entry {
    type Item = Self;

    fn serialize(&self, buf: &mut Vec<u8>) -> Result<()> {
        buf.write_all(&self.ctime.to_be_bytes())?;
        buf.write_all(&self.ctime_nsec.to_be_bytes())?;
        buf.write_all(&self.mtime.to_be_bytes())?;
        buf.write_all(&self.mtime_nsec.to_be_bytes())?;
        buf.write_all(&self.dev.to_be_bytes())?;
        buf.write_all(&self.ino.to_be_bytes())?;
        buf.write_all(&self.mode.to_be_bytes())?;
        buf.write_all(&self.uid.to_be_bytes())?;
        buf.write_all(&self.gid.to_be_bytes())?;
        buf.write_all(&self.file_size.to_be_bytes())?;
        buf.write_all(&self.sha1_obj)?;
        let name_len = self.name.len() as u16;
        buf.write_all(&name_len.to_be_bytes())?;
        buf.write_all(&self.name)?;
        // null padding
        let pad_size = calc_pad_size_for_entry(name_len);
        for _ in 0..pad_size {
            buf.push(0);
        }
        Ok(())
    }

    fn deserialize<T: Read>(reader: &mut T) -> Result<Entry> {
        let mut u32_buf = [0u8; 4];
        let mut u16_buf = [0u8; 2];
        reader.read(&mut u32_buf)?;
        let ctime = u32::from_be_bytes(u32_buf);
        reader.read(&mut u32_buf)?;
        let ctime_nsec = u32::from_be_bytes(u32_buf);
        reader.read(&mut u32_buf)?;
        let mtime = u32::from_be_bytes(u32_buf);
        reader.read(&mut u32_buf)?;
        let mtime_nsec = u32::from_be_bytes(u32_buf);
        reader.read(&mut u32_buf)?;
        let dev = u32::from_be_bytes(u32_buf);

        reader.read(&mut u32_buf)?;
        let ino = u32::from_be_bytes(u32_buf);

        reader.read(&mut u32_buf)?;
        let mode = u32::from_be_bytes(u32_buf);

        reader.read(&mut u32_buf)?;
        let uid = u32::from_be_bytes(u32_buf);

        reader.read(&mut u32_buf)?;
        let gid = u32::from_be_bytes(u32_buf);

        reader.read(&mut u32_buf)?;
        let file_size = u32::from_be_bytes(u32_buf);

        let mut sha1_obj = [0u8; 20];
        reader.read(&mut sha1_obj)?;

        reader.read(&mut u16_buf)?;
        let name_len = u16::from_be_bytes(u16_buf);

        let mut name = vec![0u8; name_len as usize];
        reader.read_exact(&mut name)?;

        let pad_size = calc_pad_size_for_entry(name_len);
        let mut pad = vec![0u8; pad_size as usize];
        reader.read_exact(&mut pad)?;

        let entry = Entry {
            ctime,
            ctime_nsec,
            mtime,
            mtime_nsec,
            dev,
            ino,
            mode,
            uid,
            gid,
            file_size,
            sha1_obj,
            name,
        };
        Ok(entry)
    }
}

fn calc_pad_size_for_entry(name_len: u16) -> u16 {
    ((8 - name_len % 8) + 2) % 8
}
