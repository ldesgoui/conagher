#![allow(unused_attributes)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![feature(concat_idents)]
#![feature(link_args)]
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
extern crate region;

use detour::{Detour, StaticDetour};
use goblin::elf::Sym;
use region::{Protection, View};
use std::collections::HashMap;
use std::ffi::CStr;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Mutex;

static_detours! {
    struct CServerGameDLL_DLLInit: fn(*const (), *const (), *const (), *const (), *const ()) -> i8;
    struct CServerGameDLL_GetTickInterval: fn() -> f32;
    struct CBaseProjectile_CanCollideWithTeammates: fn() -> i8;
    struct CTFWeaponBaseGrenadeProj_InitGrenade: extern "C" fn(*const (), *const Vector, *mut Vector, *const (), *const ());
}

lazy_static!{

static ref DLOPEN: fn(*const i8, i32) -> *const i8 = {
    badlog::init_from_env("CONAGHER_LOG"); // bit hacky but trustable
    unsafe { std::mem::transmute(libc::dlsym(libc::RTLD_NEXT, "dlopen".as_ptr() as *const i8)) }
};

static ref SYMBOLS: Mutex<HashMap<String, Sym>> = Mutex::new(HashMap::new());

static ref detour_CServerGameDLL_DLLInit: Mutex<
    StaticDetour<fn(*const (), *const (), *const (), *const (), *const ()) -> i8>,
> = Mutex::new(unsafe {
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

static ref detour_CServerGameDLL_GetTickInterval: Mutex<StaticDetour<fn() -> f32>> =
    Mutex::new(unsafe {
        CServerGameDLL_GetTickInterval
            .initialize(
                std::mem::transmute(symbol("_ZNK14CServerGameDLL15GetTickIntervalEv").unwrap()),
                || 0.0078125, // 1/2^7
            )
            .unwrap()
    });

static ref detour_CBaseProjectile_CanCollideWithTeammates: Mutex<StaticDetour<fn() -> i8>> =
    Mutex::new(unsafe {
        CBaseProjectile_CanCollideWithTeammates
            .initialize(
                std::mem::transmute(
                    symbol("_ZNK15CBaseProjectile23CanCollideWithTeammatesEv").unwrap(),
                ),
                || 0,
            )
            .unwrap()
    });

static ref detour_CTFWeaponBaseGrenadeProj_InitGrenade: Mutex<
    StaticDetour<extern "C" fn(*const (), *const Vector, *mut Vector, *const (), *const ())>,
> = Mutex::new(unsafe {
    CTFWeaponBaseGrenadeProj_InitGrenade
        .initialize(
            std::mem::transmute(
                symbol("_ZN24CTFWeaponBaseGrenadeProj11InitGrenadeERK6VectorS2_P20CBaseCombatCharacterRK13CTFWeaponInfo").unwrap(),
            ),
            |this, a, b, c, d| {
                (*b).y = 0.0;
                CTFWeaponBaseGrenadeProj_InitGrenade
                    .get()
                    .unwrap()
                    .call(this, a, b, c, d)
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

    goblin::elf::Elf::parse(&buffer)
        .map(|elf| {
            info!("dlopen: processing {:?}", path.file_name().unwrap());
            let mut map = SYMBOLS.try_lock().unwrap();
            elf.syms
                .iter()
                .map(|mut sym| {
                    sym.st_value += library_addr as u64;
                    assert!(
                        sym.st_value <= std::usize::MAX as u64,
                        "symbol points to 64bit address"
                    );
                    map.insert(elf.strtab.get_unsafe(sym.st_name).unwrap().to_string(), sym)
                })
                .last();
        })
        .unwrap_or_else(|_| {
            warn!(
                "dlopen: failed to parse ELF32 for {:?}",
                path.file_name().unwrap()
            )
        });

    debug!("{:?}", std::mem::size_of::<Vector>());

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
        detour_CBaseProjectile_CanCollideWithTeammates
            .try_lock()
            .unwrap()
            .enable()
            .unwrap();
        detour_CTFWeaponBaseGrenadeProj_InitGrenade
            .try_lock()
            .unwrap()
            .enable()
            .unwrap();

        let vptr =
            (symbol("_ZTV25CTFProjectile_HealingBolt").unwrap() as *mut *mut usize).offset(225);
        let mut view = View::new(vptr as *const u8, 4).unwrap();
        view.exec_with_prot(Protection::ReadWriteExecute, || {
            std::ptr::write(
                vptr,
                CTFProjectile_HealingBolt_CanCollideWithTeammates as *mut usize,
            );
        }).unwrap();
    };

    handle
}

#[repr(C)]
#[derive(Debug)]
pub struct Vector {
    x: f32,
    y: f32,
    z: f32,
}

extern "C" fn CTFProjectile_HealingBolt_CanCollideWithTeammates() -> u8 {
    1
}

fn symbol(name: &str) -> Option<*const ()> {
    SYMBOLS
        .lock()
        .unwrap()
        .get(name)
        .map(|sym| unsafe { std::mem::transmute(sym.st_value as usize) })
}
