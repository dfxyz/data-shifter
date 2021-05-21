use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use rand::Rng;

#[inline]
fn usage() {
    println!("Usage: data-shifter [dir] --shift file...");
    println!("       data-shifter [dir] --recover file...");
    println!("       data-shifter [dir] --force-shift file...");
    println!("       data-shifter [dir] --force-recover file...");
}

const MAGIC_NUM: &[u8; 7] = b"SHIFTED";

fn main() {
    let mut args = std::env::args().skip(1);
    let dir;
    let mode;
    match args.next() {
        None => {
            usage();
            return;
        }
        Some(s) => {
            if s.starts_with('-') {
                dir = std::env::current_dir().unwrap();
                mode = s;
            } else {
                dir = PathBuf::from(s);
                mode = match args.next() {
                    None => {
                        usage();
                        return;
                    }
                    Some(s) => s,
                };
            }
        }
    };

    if args.len() == 0 {
        usage();
        return;
    }

    std::fs::create_dir_all(&dir).unwrap();

    match mode.as_str() {
        "--shift" => {
            shift(dir, args, false);
        }
        "--restore" => {
            restore(dir, args, false);
        }
        "--force-shift" => {
            shift(dir, args, true);
        }

        "--force-restore" => {
            restore(dir, args, true);
        }
        _ => {
            usage();
            return;
        }
    }
}

fn shift<I: Iterator<Item = String>>(dir: PathBuf, mut args: I, force: bool) {
    let mut rng = rand::thread_rng();
    let mut buffer = vec![0u8; 4096];
    while let Some(p) = args.next() {
        let file = match File::open(&p) {
            Err(_) => {
                eprintln!("ignore invalid file '{}'", p);
                continue;
            }
            Ok(f) => f,
        };
        let path = PathBuf::from(&p);
        let name = path.file_name().unwrap();
        let output_path = dir.join(format!("{}.shift", name.to_string_lossy()));
        let output_file = if force {
            match OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&output_path)
            {
                Err(_) => {
                    eprintln!(
                        "ignore file '{}'; failed to open file '{}' to write",
                        p,
                        output_path.to_string_lossy()
                    );
                    continue;
                }
                Ok(f) => f,
            }
        } else {
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&output_path)
            {
                Err(_) => {
                    eprintln!(
                        "ignore file '{}'; failed to create file '{}' to write",
                        p,
                        output_path.to_string_lossy()
                    );
                    continue;
                }
                Ok(f) => f,
            }
        };

        let random = rng.gen_range(1..=u8::MAX);
        let mut reader = BufReader::new(file);
        let mut writer = BufWriter::new(output_file);

        writer.write_all(MAGIC_NUM).unwrap();
        writer.write_all(&random.to_le_bytes()).unwrap();
        let original_name = name.to_string_lossy();
        let original_name = original_name.as_bytes();
        let original_name_len = u8::try_from(original_name.len()).unwrap();
        writer.write_all(&original_name_len.to_le_bytes()).unwrap();
        writer.write_all(original_name).unwrap();

        loop {
            let num = reader.read(&mut buffer).unwrap();
            if num == 0 {
                break;
            }
            buffer[..num]
                .iter_mut()
                .for_each(|byte| *byte = u8::wrapping_add(*byte, random));
            writer.write_all(&buffer[..num]).unwrap();
        }
    }
}

fn restore<I: Iterator<Item = String>>(dir: PathBuf, mut args: I, force: bool) {
    let mut buffer = vec![0u8; 4096];
    while let Some(p) = args.next() {
        let file = match File::open(&p) {
            Err(_) => {
                eprintln!("ignore invalid file '{}'", p);
                continue;
            }
            Ok(f) => f,
        };
        let mut reader = BufReader::new(file);
        let mut header = [0u8; 8];
        if let Err(_) = reader.read_exact(&mut header) {
            eprintln!("ignore invalid file '{}'", p);
            continue;
        }
        if &header[..7] != MAGIC_NUM {
            eprintln!("ignore invalid file '{}'", p);
            continue;
        }
        let random = header[7];
        let mut original_name_len = 0u8.to_le_bytes();
        if let Err(_) = reader.read_exact(&mut original_name_len) {
            eprintln!("ignore invalid file '{}'", p);
            continue;
        }
        let original_name_len = u8::from_le_bytes(original_name_len);
        if original_name_len == 0 {
            eprintln!("ignore invalid file '{}'", p);
            continue;
        }
        let mut original_name = vec![0u8; original_name_len as usize];
        if let Err(_) = reader.read_exact(&mut original_name) {
            eprintln!("ignore invalid file '{}'", p);
            continue;
        }
        let original_name = String::from_utf8_lossy(&original_name).to_string();
        let output_path = dir.join(original_name);
        let output_file = if force {
            match OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&output_path)
            {
                Err(_) => {
                    eprintln!(
                        "ignore file '{}'; failed to open file '{}' to write",
                        p,
                        output_path.to_string_lossy()
                    );
                    continue;
                }
                Ok(f) => f,
            }
        } else {
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&output_path)
            {
                Err(_) => {
                    eprintln!(
                        "ignore file '{}'; failed to create file '{}' to write",
                        p,
                        output_path.to_string_lossy()
                    );
                    continue;
                }
                Ok(f) => f,
            }
        };
        let mut writer = BufWriter::new(output_file);

        loop {
            let num = reader.read(&mut buffer).unwrap();
            if num == 0 {
                break;
            }
            buffer[..num]
                .iter_mut()
                .for_each(|byte| *byte = u8::wrapping_sub(*byte, random));
            writer.write_all(&buffer[..num]).unwrap();
        }
    }
}
