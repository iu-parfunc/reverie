use libc;
use nix::sys::wait::WaitStatus;
use nix::sys::{ptrace, signal, uio, wait};
use nix::unistd;
use nix::unistd::Pid;
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use std::ptr::NonNull;

use crate::consts;
use crate::consts::*;
use crate::hooks;
use crate::nr;
use crate::proc::*;
use crate::stubs;
use crate::nr::SyscallNo::*;
use crate::task::Task;
use crate::traced_task::TracedTask;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyscallStubPage {
    pub address: u64,
    pub size: usize,
    pub allocated: usize,
}

#[derive(Debug)]
pub struct RemotePtr<T> {
    ptr: NonNull<T>,
}

impl<T> RemotePtr<T>
where
    T: Sized,
{
    pub fn new(not_null: NonNull<T>) -> Self {
        RemotePtr { ptr: not_null }
    }
    pub fn as_ptr(self) -> *mut T {
        self.ptr.as_ptr()
    }
    pub fn cast<U>(self) -> RemotePtr<U> {
        RemotePtr {
            ptr: self.ptr.cast::<U>(),
        }
    }
}

impl<T> Clone for RemotePtr<T> {
    fn clone(&self) -> Self {
        RemotePtr {
            ptr: self.ptr.clone(),
        }
    }
}

impl<T: Sized> Copy for RemotePtr<T> {}

pub trait RemoteSyscall {
    fn untraced_syscall(
        &mut self,
        nr: nr::SyscallNo,
        a0: i64,
        a1: i64,
        a2: i64,
        a3: i64,
        a4: i64,
        a5: i64,
    ) -> Result<i64>;
    fn traced_syscall(
        &mut self,
        nr: nr::SyscallNo,
        a0: i64,
        a1: i64,
        a2: i64,
        a3: i64,
        a4: i64,
        a5: i64,
    ) -> Result<i64>;
}

pub trait Remote {
    fn peek_bytes(&self, addr: RemotePtr<u8>, size: usize) -> Result<Vec<u8>>;
    fn poke_bytes(&self, addr: RemotePtr<u8>, bytes: &[u8]) -> Result<()>;
    fn peek<T>(&self, addr: RemotePtr<T>) -> Result<T>
    where
        T: Sized,
    {
        let new_ptr = addr.cast::<u8>();
        let size = std::mem::size_of::<T>();
        let bytes: Vec<u8> = self.peek_bytes(new_ptr, size)?;
        // to be initialized by copy_nonoverlapping.
        let mut res: T = unsafe { std::mem::uninitialized() };
        let ret_ptr: *mut T = &mut res;
        unsafe { std::ptr::copy(bytes.as_ptr(), ret_ptr as *mut u8, size) };
        Ok(res)
    }
    fn poke<T>(&self, addr: RemotePtr<T>, value: &T) -> Result<()>
    where
        T: Sized,
    {
        let value_ptr: *const T = value;
        let size = std::mem::size_of::<T>();
        let new_ptr = addr.cast::<u8>();
        let bytes: &[u8] = unsafe {
            let raw_bytes = std::mem::transmute(value_ptr as *const u8);
            std::slice::from_raw_parts(raw_bytes, size)
        };
        self.poke_bytes(new_ptr, bytes)?;
        Ok(())
    }
    fn getregs(&self) -> Result<libc::user_regs_struct>;
    fn setregs(&self, regs: libc::user_regs_struct) -> Result<()>;
    fn getevent(&self) -> Result<i64>;
    fn resume(&self, sig: Option<signal::Signal>) -> Result<()>;
}

fn ensure_syscall(pid: unistd::Pid, rip: u64) -> Result<()> {
    let insn = ptrace::read(pid, rip as ptrace::AddressType).expect("ptrace peek failed") as u64;
    match insn & SYSCALL_INSN_MASK as u64 {
        SYSCALL_INSN => Ok(()),
        _otherwise => Err(Error::new(
            ErrorKind::Other,
            format!(
                "expect syscall instructions at {:x}, but got: {:x}",
                rip, insn
            ),
        )),
    }
}

fn synchronize_from(task: &mut TracedTask, rip: u64) -> Result<()> {
    std::thread::sleep(std::time::Duration::from_micros(1000));
    Ok(())
}

pub fn patch_at(
    task: &mut TracedTask,
    hook: &hooks::SyscallHook,
    target: u64,
) -> Result<()> {
    let jmp_insn_size = 5;
    let regs = task.getregs()?;
    let resume_from = regs.rip - SYSCALL_INSN_SIZE as u64;
    let ip = resume_from;
    let rela: i64 = target as i64 - ip as i64 - jmp_insn_size as i64;
    assert!(rela >= -1i64.wrapping_shl(31) && rela < 1i64.wrapping_shl(31));

    let mut patch_bytes: Vec<u8> = Vec::new();

    let remote_rip = RemotePtr::new(NonNull::new(ip as *mut u8).unwrap());

    patch_bytes.push(0xe8);
    patch_bytes.push((rela & 0xff) as u8);
    patch_bytes.push((rela.wrapping_shr(8) & 0xff) as u8);
    patch_bytes.push((rela.wrapping_shr(16) & 0xff) as u8);
    patch_bytes.push((rela.wrapping_shr(24) & 0xff) as u8);

    let padding_size = SYSCALL_INSN_SIZE + hook.instructions.len() - jmp_insn_size as usize;
    assert!(padding_size <= 9);

    match padding_size {
        0 => (),
        1 => patch_bytes.push(0x90),
        2 => {
            patch_bytes.push(0x66);
            patch_bytes.push(0x90);
        }
        3 => {
            patch_bytes.push(0x0f);
            patch_bytes.push(0x1f);
            patch_bytes.push(0x00);
        }
        4 => {
            patch_bytes.push(0x0f);
            patch_bytes.push(0x1f);
            patch_bytes.push(0x40);
            patch_bytes.push(0x00);
        }
        5 => {
            patch_bytes.push(0x0f);
            patch_bytes.push(0x1f);
            patch_bytes.push(0x44);
            patch_bytes.push(0x00);
            patch_bytes.push(0x00);
        }
        6 => {
            patch_bytes.push(0x66);
            patch_bytes.push(0x0f);
            patch_bytes.push(0x1f);
            patch_bytes.push(0x44);
            patch_bytes.push(0x00);
            patch_bytes.push(0x00);
        }
        7 => {
            patch_bytes.push(0x0f);
            patch_bytes.push(0x1f);
            patch_bytes.push(0x80);
            patch_bytes.push(0x00);
            patch_bytes.push(0x00);
            patch_bytes.push(0x00);
            patch_bytes.push(0x00);
        }
        8 => {
            patch_bytes.push(0x0f);
            patch_bytes.push(0x1f);
            patch_bytes.push(0x84);
            patch_bytes.push(0x00);
            patch_bytes.push(0x00);
            patch_bytes.push(0x00);
            patch_bytes.push(0x00);
            patch_bytes.push(0x00);
        }
        9 => {
            patch_bytes.push(0x66);
            patch_bytes.push(0x0f);
            patch_bytes.push(0x1f);
            patch_bytes.push(0x84);
            patch_bytes.push(0x00);
            patch_bytes.push(0x00);
            patch_bytes.push(0x00);
            patch_bytes.push(0x00);
            patch_bytes.push(0x00);
        }
        _ => panic!("maximum padding is 9"),
    };
    assert_eq!(patch_bytes.len(), hook.instructions.len() + consts::SYSCALL_INSN_SIZE);
    let page = ip & !0xfff;
    // must perform check when patch across page boundry
    let size = if ((ip as usize + patch_bytes.len()) & !0xfff) < patch_bytes.len() {
        0x2000
    } else {
        0x1000
    };
    task.untraced_syscall(
        SYS_mprotect,
        page as i64,
        size as i64,
        libc::PROT_WRITE as i64,
        0, 0, 0).expect(&format!("mprotect failed page: {:x}, size: {:x}", page, size));
    task.poke_bytes(remote_rip, patch_bytes.as_slice())?;
    task.untraced_syscall(
        SYS_mprotect,
        page as i64,
        size as i64,
        (libc::PROT_READ | libc::PROT_EXEC) as i64,
        0, 0, 0).expect(&format!("mprotect failed page: {:x}, size: {:x}", page, size));
    let mut new_regs = regs.clone();
    new_regs.rax = regs.orig_rax; // for our patch, we use rax as syscall no.
    new_regs.rip = ip;            // rewind pc back (-2).
    task.setregs(new_regs)?;
    // because we modified tracee's code
    // we need some kind of synchronization to make sure
    // the CPU (especially i-cache) noticed the change
    // hence we set a breakponit at ip (original rip - 2)
    // to force synchronization.
    synchronize_from(task, ip)
}

// search for spare page(s) which can be allocated (mmap) within the
// range of @addr_hint +/- 2GB.
pub fn search_stub_page(pid: Pid, addr_hint: u64, pages: usize) -> Result<u64> {
    let mappings = decode_proc_maps(pid)?;
    let page_size: u64 = 0x1000;
    let one_mb: u64 = 0x100000;
    let almost_2gb: u64 = 2u64.wrapping_shl(30) - 0x100000;
    let mut ranges_from: Vec<(u64, u64)> = Vec::new();
    let mut ranges_to: Vec<(u64, u64)> = Vec::new();

    ranges_from.push((one_mb - page_size, one_mb));
    mappings
        .iter()
        .for_each(|e| ranges_from.push((e.base(), e.end())));
    mappings
        .iter()
        .for_each(|e| ranges_to.push((e.base(), e.end())));
    ranges_to.push((0xffffffff_ffff_8000u64, 0xffffffff_ffff_f000u64));
    debug_assert_eq!(ranges_from.len(), ranges_to.len());

    let res: Vec<u64> = ranges_from
        .iter()
        .zip(ranges_to)
        .filter_map(|((x1, y1), (x2, y2))| {
            let space = x2 - y1;
            let start_from = *y1;
            if space >= (pages as u64 * page_size) {
                if start_from <= addr_hint && start_from + almost_2gb >= addr_hint {
                    Some(start_from)
                } else if start_from >= addr_hint
                    && start_from - addr_hint <= almost_2gb - (pages as u64 * page_size)
                {
                    Some(start_from)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    match res.iter().next() {
        None => Err(Error::new(
            ErrorKind::Other,
            format!("cannot allocate stub page for {:x}", addr_hint),
        )),
        Some(addr) => Ok(*addr),
    }
}

#[test]
fn can_find_stub_page() {
    let pid = unistd::getpid();
    let ranges: Vec<(u64, u64)> = decode_proc_maps(pid)
        .unwrap()
        .iter()
        .map(|e| (e.base(), e.end()))
        .collect();
    let addr_hints: Vec<u64> = decode_proc_maps(pid)
        .unwrap()
        .iter()
        .map(|e| e.base() + 0x234)
        .collect();
    let two_gb = 2u64.wrapping_shl(30);
    for hint in addr_hints {
        let ret_ = search_stub_page(pid, hint, 1);
        assert!(ret_.is_ok());
        let ret = ret_.unwrap();
        println!("searching {:x} returned {:x}", hint, ret);
        if ret <= hint {
            assert!(hint - ret <= two_gb);
        } else {
            assert!(ret - hint <= two_gb);
        }
        let has_collision = ranges.iter().fold(false, |acc, (start, end)| {
            if acc {
                acc
            } else {
                ret >= *start && ret < *end
            }
        });
        assert!(!has_collision);
    }
}

// generate syscall instructions at injected page
// the page address should be 0x7000_0000
// the byte code can be confirmed by running objcopy
// x86_64-linux-gnu-objcopy -I binary /tmp/1.bin -O elf64-x86-64 -B i386:x86-64 /tmp/1.elf
// then objdump -d 1.elf must match the instructions listed below.
pub fn gen_syscall_sequences_at(pid: Pid, page_address: u64) -> nix::Result<()> {
    /* the syscall sequences used here:
     * 0:   0f 05                   syscall
     * 2:   c3                      retq                     // not filered by seccomp, untraced_syscall
     * 3:   90                      nop
     * 4:   0f 05                   syscall                  // traced syscall
     * 6:   c3                      retq
     * 7:   90                      nop
     * 8:   e8 f3 ff ff ff          callq  0 <_do_syscall>   // untraced syscall, then breakpoint.
     * d:   cc                      int3
     * e:   66 90                   xchg   %ax,%ax
     * 10:  e8 ef ff ff ff          callq  4 <_do_syscall+0x4> // traced syscall, then breakpoint
     * 15:  cc                      int3
     * 16:  66 90                   xchg   %ax,%ax
     */
    let syscall_stub: &[u64] = &[0x90c3050f90c3050f, 0x9066ccfffffff3e8, 0x9066ccffffffefe8];
    // please note we force each `ptrace::write` to be exactly ptrace_poke (8 bytes a time)
    // instead of using `process_vm_writev`, because this function can be called in
    // PTRACE_EXEC_EVENT, the process seems not fully loaded by ld-linux.so
    // call process_vm_{readv, writev} would 100% fail.
    for (k, s) in syscall_stub.iter().enumerate() {
        let offset = k * std::mem::size_of::<u64>() + page_address as usize;
        ptrace::write(pid, offset as ptrace::AddressType, *s as *mut libc::c_void)?;
    }
    Ok(())
}
