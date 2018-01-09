#![feature(link_args)]
#![allow(unused_attributes)]
#![link_args = "-Wl,-z,defs -Wl,--no-undefined -ldl"]

extern crate goblin;
#[macro_use]
extern crate lazy_static;
extern crate libc;

use std::sync::Mutex;
use goblin::elf::Elf;
use libc::{c_char, c_int, c_void};
use std::collections::HashSet;
use std::ffi::CStr;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

lazy_static! {
    static ref DLOPEN: fn(*const c_char, c_int) -> *mut c_void = unsafe {
        let sym = libc::dlsym(libc::RTLD_NEXT, "dlopen".as_ptr() as *const c_char);
        if sym.is_null() {
            panic!("{:?}", CStr::from_ptr(libc::dlerror()).to_str());
        }
        std::mem::transmute(sym)
    };

    static ref LIBS_WANTED: HashSet<&'static str> = [
        "dedicated_srv.so",
        "engine_srv.so",
        "filesystem_stdio.so",
        "libtier0_srv.so",
        "replay_srv.so",
        "server_srv.so",
        "soundemittersystem_srv.so",
    ].iter().cloned().collect();

    static ref LIBRARIES: Mutex<Vec<Library<'static>>> = Mutex::new(Vec::new());
}

#[no_mangle]
#[allow(unused_must_use)]
pub extern "C" fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void {
    let handle = (DLOPEN)(filename, flags);
    if handle.is_null() {
        return handle;
    }

    let safe_filename = unsafe { CStr::from_ptr(filename).to_string_lossy().into_owned() };

    std::panic::catch_unwind(|| {
        lib_hook(safe_filename, handle as *mut ());
    });

    handle
}

fn lib_hook(filename: String, handle: *mut ()) {
    let mut path = PathBuf::from("bin"); // FIXME
    path.push(filename);

    if !LIBS_WANTED.contains(&path.file_name().unwrap().to_str().unwrap()) {
        return;
    }

    let mut buffer = Vec::new();
    let mut fd = File::open(path.clone()).unwrap();
    fd.read_to_end(&mut buffer).unwrap();
    Library::new(path, Elf::parse(&buffer).unwrap(), handle);
}

#[derive(Debug)]
struct Library<'a> {
    path: PathBuf,
    elf: Elf<'a>,
    ptr: *mut (),
}

unsafe impl<'a> Send for Library<'a> {}
unsafe impl<'a> Sync for Library<'a> {}

impl<'a> Library<'a> {
    fn new(path: PathBuf, elf: Elf<'a>, ptr: *mut ()) -> Library<'a> {
        Library {
            path: path,
            elf: elf,
            ptr: ptr,
        }
    }
}
