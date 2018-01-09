#![allow(unused_attributes)]
#![allow(non_upper_case_globals)]
#![feature(link_args)]
#![feature(pointer_methods)]
#![link_args = "-Wl,-z,defs -Wl,--no-undefined -ldl"]

extern crate badlog;
#[macro_use]
extern crate detour;
extern crate goblin;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate log;
extern crate region;

use region::Protection;
use goblin::elf::Elf;
use detour::Detour;
use std::ffi::CStr;
use std::path::PathBuf;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;

lazy_static! {
    static ref DLOPEN: fn(*const i8, i32) -> *mut u8 = unsafe {
        badlog::init_from_env("TF2RS_LOG"); // bit hacky but trustable
        std::mem::transmute(libc::dlsym(libc::RTLD_NEXT, "dlopen".as_ptr() as *const i8))
    };
}

const CServerGameDLL_DLLInit: &'static str =
    "_ZN14CServerGameDLL7DLLInitEPFPvPKcPiES5_S5_P11CGlobalVars";

static_detours! {
    struct DLLInit: extern "C" fn(*const (), *const (), *const (), *const ()) -> i8;
}

#[no_mangle]
pub extern "C" fn dlopen(filename: *const i8, flags: i32) -> *mut u8 {
    let handle = (DLOPEN)(filename, flags);
    if handle.is_null() {
        return handle;
    }

    let mut path = PathBuf::from("bin");
    path.push(unsafe { CStr::from_ptr(filename) }.to_str().unwrap());
    if !path.is_file() {
        warn!("dlopen-ed but inexistant: {}", path.display());
        return handle;
    }

    info!("processing: {:?}", path.file_name().unwrap());
    let mut buffer = Vec::new();
    let mut fd = File::open(path).unwrap();
    fd.read_to_end(&mut buffer).unwrap();

    let elf = Elf::parse(&buffer).unwrap();
    let map: HashMap<&str, goblin::elf::sym::Sym> = elf.syms
        .iter()
        .map(|sym| (elf.strtab.get(sym.st_name).unwrap().unwrap(), sym))
        .collect();

    match map.get(CServerGameDLL_DLLInit) {
        None => (),
        Some(sym) => unsafe {
            let base = *(handle as *mut *mut u8);
            let ptr = base.add(sym.st_value as usize);
            region::protect(ptr, sym.st_size as usize, Protection::ReadWriteExecute).expect("rwx");
            let mut hook = DLLInit
                .initialize(std::mem::transmute(ptr), |a, b, c, d| {
                    error!("WE IN BOYS AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
                    DLLInit.get().expect("idfk").call(a, b, c, d)
                })
                .expect("help");
            hook.enable().expect("help2");
        },
    };

    handle
}
