/*
 * Copyright (c) 2018-2019, Trustees of Indiana University
 *     ("University Works" via Baojun Wang)
 * Copyright (c) 2018-2019, Ryan Newton
 *     ("Traditional Works of Scholarship")
 *
 *  All rights reserved.
 *
 *  This source code is licensed under the BSD-style license found in the
 *  LICENSE file in the root directory of this source tree.
 */

pub const REVERIE_TRACEE_PRELOAD: &str = "REVERIE_TRACEE_PRELOAD";

pub const REVERIE_ENV_TOOL_LOG_KEY: &str = "TOOL_LOG";

pub const SYSCALL_INSN_SIZE: usize = 2;
pub const SYSCALL_INSN_MASK: u64 = 0xffff;
pub const SYSCALL_INSN: u64 = 0x050f;

pub const REVERIE_PRIVATE_PAGE_OFFSET: u64 = 0x7000_0000;
pub const REVERIE_PRIVATE_PAGE_SIZE: u64 = 0x4000;

pub const REVERIE_GLOBAL_STATE_FILE: &str = "reverie";
pub const REVERIE_GLOBAL_STATE_ADDR: u64 = 0x7020_0000;
pub const REVERIE_GLOBAL_STATE_SIZE: u64 = 0x1000;
pub const REVERIE_GLOBAL_STATE_FD: i32 = 1023;

pub const REVERIE_DPC_SOCKFD: i32 = 1022;

pub const REVERIE_LOCAL_BASE: u64 = REVERIE_PRIVATE_PAGE_OFFSET + 0x1000;

pub const REVERIE_LOCAL_SYSCALL_HOOK_SIZE: u64 = REVERIE_LOCAL_BASE;
pub const REVERIE_LOCAL_SYSCALL_HOOK_ADDR: u64 =
    REVERIE_LOCAL_SYSCALL_HOOK_SIZE + core::mem::size_of::<u64>() as u64;

pub const REVERIE_LOCAL_STUB_SCRATCH: u64 =
    REVERIE_LOCAL_SYSCALL_HOOK_ADDR + core::mem::size_of::<u64>() as u64;
pub const REVERIE_LOCAL_STACK_NESTING_LEVEL: u64 =
    REVERIE_LOCAL_STUB_SCRATCH + core::mem::size_of::<u64>() as u64;

pub const REVERIE_LOCAL_SYSCALL_TRAMPOLINE: u64 =
    REVERIE_LOCAL_STACK_NESTING_LEVEL + core::mem::size_of::<u64>() as u64;
pub const REVERIE_LOCAL_SYSTOOL_HOOK: u64 =
    REVERIE_LOCAL_SYSCALL_TRAMPOLINE + core::mem::size_of::<u64>() as u64;
pub const REVERIE_LOCAL_SYSCALL_PATCH_LOCK: u64 =
    REVERIE_LOCAL_SYSTOOL_HOOK + core::mem::size_of::<u64>() as u64;

pub const REVERIE_LOCAL_SYSTOOL_LOG_LEVEL: u64 =
    REVERIE_LOCAL_SYSCALL_PATCH_LOCK + core::mem::size_of::<u64>() as u64;

pub const REVERIE_LOCAL_REVERIE_LOCAL_STATE: u64 =
    REVERIE_LOCAL_SYSTOOL_LOG_LEVEL + core::mem::size_of::<u64>() as u64;

pub const REVERIE_LOCAL_REVERIE_GLOBAL_STATE: u64 =
    REVERIE_LOCAL_REVERIE_LOCAL_STATE + core::mem::size_of::<u64>() as u64;

pub const REVERIE_LOCAL_SYSCALL_HELPER: u64 =
    REVERIE_LOCAL_REVERIE_GLOBAL_STATE + core::mem::size_of::<u64>() as u64;

pub const REVERIE_LOCAL_RPC_HELPER: u64 =
    REVERIE_LOCAL_SYSCALL_HELPER + core::mem::size_of::<u64>() as u64;

pub const REVERIE_LOCAL_DPC_FUTEX: u64 =
    REVERIE_LOCAL_RPC_HELPER + core::mem::size_of::<u64>() as u64;

pub const REVERIE_LOCAL_TLS_GET_ADDR_OFFSET: u64 =
    REVERIE_LOCAL_DPC_FUTEX + core::mem::size_of::<u64>() as u64;

#[test]
fn det_tls_sanity_check() {
    assert_eq!(REVERIE_LOCAL_SYSCALL_HOOK_SIZE, REVERIE_LOCAL_BASE + 0);
    assert_eq!(REVERIE_LOCAL_SYSCALL_HOOK_ADDR, REVERIE_LOCAL_BASE + 8);
    assert_eq!(REVERIE_LOCAL_STUB_SCRATCH, REVERIE_LOCAL_BASE + 16);
    assert_eq!(REVERIE_LOCAL_STACK_NESTING_LEVEL, REVERIE_LOCAL_BASE + 24);
    assert_eq!(REVERIE_LOCAL_SYSCALL_TRAMPOLINE, REVERIE_LOCAL_BASE + 32);
    assert_eq!(REVERIE_LOCAL_SYSTOOL_HOOK, REVERIE_LOCAL_BASE + 40);
    assert_eq!(REVERIE_LOCAL_SYSCALL_PATCH_LOCK, REVERIE_LOCAL_BASE + 48);
    assert_eq!(REVERIE_LOCAL_SYSTOOL_LOG_LEVEL, REVERIE_LOCAL_BASE + 56);
    assert_eq!(REVERIE_LOCAL_REVERIE_LOCAL_STATE, REVERIE_LOCAL_BASE + 64);
    assert_eq!(REVERIE_LOCAL_REVERIE_GLOBAL_STATE, REVERIE_LOCAL_BASE + 72);
    assert_eq!(REVERIE_LOCAL_SYSCALL_HELPER, REVERIE_LOCAL_BASE + 80);
    assert_eq!(REVERIE_LOCAL_RPC_HELPER, REVERIE_LOCAL_BASE + 88);
    assert_eq!(REVERIE_LOCAL_DPC_FUTEX, REVERIE_LOCAL_BASE + 96);
    assert_eq!(REVERIE_LOCAL_TLS_GET_ADDR_OFFSET, REVERIE_LOCAL_BASE + 104);
}
