use glob::glob;
use std::env;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

pub struct HookBinaryPaths {
    pub exe32: PathBuf,
    pub exe64: PathBuf,
    pub dll32: PathBuf,
    pub dll64: PathBuf,
}

impl HookBinaryPaths {
    pub fn get() -> Self {
        // Find top level directory by scanning up for Cargo.toml
        let mut path = env::current_exe().unwrap();
        loop {
            if !path.pop() {
                panic!("Could not find Cargo.toml");
            }
            if path.join("Cargo.toml").exists() {
                break;
            }
        }
        let mut exe32 = None;
        let mut exe64 = None;
        let mut dll32 = None;
        let mut dll64 = None;
        for e in glob(&format!("{}/**/wlw-hook*.exe", path.to_str().unwrap())).unwrap() {
            let entry = e.unwrap();
            let mut file = File::open(&entry).unwrap();
            let mut offset_buf = [0; 1];
            file.seek(SeekFrom::Start(0x3C)).unwrap();
            file.read_exact(&mut offset_buf).unwrap();
            let offset = u64::from(offset_buf[0]);
            file.seek(SeekFrom::Start(offset + 4)).unwrap();
            let mut code_buf = [0; 2];
            file.read_exact(&mut code_buf).unwrap();
            let code = u16::from(code_buf[0]) | (u16::from(code_buf[1]) << 8);
            match code {
                0x8664 => exe64 = Some(entry),
                0x014c => exe32 = Some(entry),
                _ => panic!(
                    "\"{}\" is invalid, offset {:08x}, code {:04x}",
                    entry.to_str().unwrap(),
                    offset,
                    code
                ),
            }
        }
        for e in glob(&format!("{}/**/wlw_hook*.dll", path.to_str().unwrap())).unwrap() {
            let entry = e.unwrap();
            if entry.ends_with("wlw_hook32.dll") {
                dll32 = Some(entry);
            } else if entry.ends_with("wlw_hook64.dll") {
                dll64 = Some(entry);
            } else {
                panic!("\"{}\" is invalid", entry.to_str().unwrap());
            }
        }
        HookBinaryPaths {
            exe32: exe32.unwrap(),
            exe64: exe64.unwrap(),
            dll32: dll32.unwrap(),
            dll64: dll64.unwrap(),
        }
    }
}
