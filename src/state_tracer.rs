
use nix::sys::mman::{ProtFlags, MapFlags, mmap, munmap};
use nix::sys::memfd::{memfd_create, MemFdCreateFlag};
use nix::fcntl::{SealFlag, fcntl};
use nix::unistd;
use std::cell::UnsafeCell;
use std::sync::Once;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::ptr::NonNull;
use std::ffi::CStr;
use crate::consts;
use crate::state::SystraceState;

pub fn systrace_state_allocate() -> *mut SystraceState {
    unsafe {
        let size = consts::SYSTRACE_GLOBAL_STATE_SIZE as usize;
        // no CLOEXEC
        let fd0 = memfd_create(CStr::from_ptr(
            consts::SYSTRACE_GLOBAL_STATE_FILE.as_ptr() as *const i8),
                               MemFdCreateFlag::empty()).unwrap();
        let fd = consts::SYSTRACE_GLOBAL_STATE_FD;
        unistd::dup2(fd0, fd).unwrap();
        unistd::close(fd0).unwrap();
        unistd::ftruncate(fd, size as i64).unwrap();
        let void_p = mmap(0 as *mut _, size,
                          ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                          MapFlags::MAP_SHARED | MapFlags::MAP_ANONYMOUS,
                          fd, 0).unwrap();
        void_p as *mut SystraceState
    }
}

pub static mut SYSTRACE_STATE: Option<NonNull<SystraceState>> = None;

#[link_section = ".init_array"]
static INITIALIZER: extern "C" fn() = rust_stats_ctor;

#[no_mangle]
extern "C" fn rust_stats_ctor() {
    unsafe {
        let ptr = systrace_state_allocate();
        SYSTRACE_STATE = Some(NonNull::new_unchecked(ptr));
    }
}

pub fn get_systrace_state() -> &'static mut SystraceState {
    unsafe {
        let ptr = SYSTRACE_STATE.expect("SYSTRACE_STATE not initialized");
        let state = &mut *ptr.as_ptr();
        state
    }
}
