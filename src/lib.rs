#![allow(unused_attributes)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![feature(concat_idents)]
#![feature(link_args)]
#![feature(pointer_methods)]
#![link_args = "-Wl,-z,defs -Wl,--no-undefined -ldl"]

extern crate badlog;
extern crate cpp_demangle;
#[macro_use]
extern crate detour;
extern crate goblin;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate log;

use detour::Detour;
use detour::StaticDetour;
use goblin::elf::Elf;
use std::collections::HashMap;
use std::ffi::CStr;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Mutex;

static_detours! {
    struct CServerGameDLL_DLLInit: fn(*const (), *const (), *const (), *const (), *const ()) -> i8;
    struct CServerGameDLL_GetTickInterval: fn(*const ()) -> f32;
}

lazy_static! {
    static ref DLOPEN: fn(*const i8, i32) -> *const i8 = {
        badlog::init_from_env("CONAGHER_LOG"); // bit hacky but trustable
        unsafe { std::mem::transmute(libc::dlsym(libc::RTLD_NEXT, "dlopen".as_ptr() as *const i8)) }
    };

    static ref SYMBOLS: Mutex<HashMap<String, usize>> = Mutex::new(HashMap::new());

    static ref detour_CServerGameDLL_DLLInit: Mutex<
        StaticDetour<fn(*const (), *const (), *const (), *const (), *const ()) -> i8>,
    > = Mutex::new(unsafe {
        info!("Applying detour to: {:?}", cpp_demangle::Symbol::new("_ZN14CServerGameDLL7DLLInitEPFPvPKcPiES5_S5_P11CGlobalVars").unwrap().to_string());
        CServerGameDLL_DLLInit
            .initialize(
                std::mem::transmute(
                    symbol("_ZN14CServerGameDLL7DLLInitEPFPvPKcPiES5_S5_P11CGlobalVars")
                        .expect("CServerGameDLL_DLLInit detour initialized with no symbol available"),
                ),
                |this, a, b, c, d| {
                    info!("We're in business.");
                    CServerGameDLL_DLLInit.get().unwrap().call(this, a, b, c, d)
                },
            )
            .expect("Failed to initialize DLLInit detour")
    });

    static ref detour_CServerGameDLL_GetTickInterval: Mutex<StaticDetour<fn(*const ()) -> f32>> =
        Mutex::new(unsafe {
        info!("Applying detour to: {:?}", cpp_demangle::Symbol::new("_ZNK14CServerGameDLL15GetTickIntervalEv").unwrap().to_string());
            CServerGameDLL_GetTickInterval
                .initialize(
                    std::mem::transmute(symbol("_ZNK14CServerGameDLL15GetTickIntervalEv").unwrap()),
                    |_| {
                        0.008
                    },
                )
                .unwrap()
        });
}

#[no_mangle]
pub extern "C" fn dlopen(filename: *const i8, flags: i32) -> *const i8 {
    let handle = (DLOPEN)(filename, flags);
    if handle.is_null() {
        return handle;
    }

    let mut path = PathBuf::from("bin");
    path.push(unsafe { CStr::from_ptr(filename) }.to_str().unwrap());
    // TODO
    if path.file_name().unwrap() != "server_srv.so" {
        debug!(
            "dlopen: skipping uninteresting {:?}",
            path.file_name().unwrap()
        );
        return handle;
    } else if !path.is_file() {
        warn!("dlopen of inexistant: {}", path.display());
        return handle;
    }

    let mut buffer = Vec::new();
    let mut fd = File::open(path.as_path()).unwrap();
    fd.read_to_end(&mut buffer).unwrap();

    let library_addr: usize = unsafe { *(handle as *const *const i8) as usize };

    Elf::parse(&buffer)
        .map(|elf| {
            info!("dlopen: processing {:?}", path.file_name().unwrap());
            let mut map = SYMBOLS.try_lock().unwrap();
            elf.syms
                .iter()
                .map(|sym| {
                    map.insert(
                        elf.strtab.get_unsafe(sym.st_name).unwrap().to_string(),
                        library_addr + sym.st_value as usize,
                    )
                })
                .last();
        })
        .unwrap_or_else(|_| {
            warn!(
                "dlopen: failed to parse ELF32 for {:?}",
                path.file_name().unwrap()
            )
        });

    unsafe {
        // TODO: safety (could crash if server_srv.so isnt loaded)
        detour_CServerGameDLL_DLLInit
            .try_lock()
            .unwrap()
            .enable()
            .unwrap();
        detour_CServerGameDLL_GetTickInterval
            .try_lock()
            .unwrap()
            .enable()
            .unwrap();
    };

    handle
}

fn symbol(name: &str) -> Option<*const ()> {
    SYMBOLS
        .lock()
        .unwrap()
        .get(name)
        .map(|&sym| unsafe { std::mem::transmute(sym) })
}
