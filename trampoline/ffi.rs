/// ffi.rs: re-exports trampoline symbols.
///
/// NB: rust (as of today's nightly) doesn't export symbols from .c/.S files,
/// also rust doesn't seem to have visibility controls such as
/// __attribute__((visibility("hidden"))), there's no good way to workaround
/// this, see rust issue ##36342 for more details.
/// As a result, we re-export all the needed C/ASM symbols to make sure our
/// cdylib is built correctly.

use core::ffi::c_void;

static SYSCALL_UNTRACED: u64 = 0x7000_0000;
static SYSCALL_TRACED: u64 = 0x7000_0004;

extern "C" {
    fn _raw_syscall(syscallno: i32,
                    arg0: i64,
                    arg1: i64,
                    arg2: i64,
                    arg3: i64,
                    arg4: i64,
                    arg5: i64,
                    syscall_insn: *mut c_void,
                    sp1: i64,
                    sp2: i64) -> i64;
    fn _syscall_hook_trampoline();
    fn _syscall_hook_trampoline_48_3d_01_f0_ff_ff();
    fn _syscall_hook_trampoline_48_3d_00_f0_ff_ff();
    fn _syscall_hook_trampoline_48_8b_3c_24();
    fn _syscall_hook_trampoline_5a_5e_c3();
    fn _syscall_hook_trampoline_89_c2_f7_da();
    fn _syscall_hook_trampoline_90_90_90();
    fn _syscall_hook_trampoline_ba_01_00_00_00();
    fn _syscall_hook_trampoline_89_c1_31_d2();
    fn _syscall_hook_trampoline_c3_nop();
    fn _syscall_hook_trampoline_85_c0_0f_94_c2();
}

#[no_mangle]
unsafe extern "C" fn syscall_hook_trampoline() {
    _syscall_hook_trampoline()
}

#[no_mangle]
unsafe extern "C" fn syscall_hook_trampoline_48_3d_01_f0_ff_ff() {
    _syscall_hook_trampoline_48_3d_01_f0_ff_ff()
}

#[no_mangle]
unsafe extern "C" fn syscall_hook_trampoline_48_3d_00_f0_ff_ff() {
    _syscall_hook_trampoline_48_3d_00_f0_ff_ff()
}
#[no_mangle]
unsafe extern "C" fn syscall_hook_trampoline_48_8b_3c_24() {
    _syscall_hook_trampoline_48_8b_3c_24()
}

#[no_mangle]
unsafe extern "C" fn syscall_hook_trampoline_5a_5e_c3() {
    _syscall_hook_trampoline_5a_5e_c3()
}

#[no_mangle]
unsafe extern "C" fn syscall_hook_trampoline_89_c2_f7_da() {
    _syscall_hook_trampoline_89_c2_f7_da()
}

#[no_mangle]
unsafe extern "C" fn syscall_hook_trampoline_90_90_90() {
    _syscall_hook_trampoline_90_90_90()
}

#[no_mangle]
unsafe extern "C" fn syscall_hook_trampoline_ba_01_00_00_00() {
    _syscall_hook_trampoline_ba_01_00_00_00()
}

#[no_mangle]
unsafe extern "C" fn syscall_hook_trampoline_89_c1_31_d2() {
    _syscall_hook_trampoline_89_c1_31_d2()
}

#[no_mangle]
unsafe extern "C" fn syscall_hook_trampoline_c3_nop() {
    _syscall_hook_trampoline_c3_nop()
}

#[no_mangle]
unsafe extern "C" fn syscall_hook_trampoline_85_c0_0f_94_c2() {
    _syscall_hook_trampoline_85_c0_0f_94_c2()
}

#[no_mangle]
unsafe extern "C" fn traced_syscall(
    syscallno: i32,
    arg0: i64,
    arg1: i64,
    arg2: i64,
    arg3: i64,
    arg4: i64,
    arg5: i64) -> i64 {
    _raw_syscall(syscallno, arg0, arg1, arg2, arg3, arg4, arg5,
                 SYSCALL_TRACED as *mut _, 0, 0)
}

#[no_mangle]
unsafe extern "C" fn untraced_syscall(
    syscallno: i32,
    arg0: i64,
    arg1: i64,
    arg2: i64,
    arg3: i64,
    arg4: i64,
    arg5: i64) -> i64 {
    _raw_syscall(syscallno, arg0, arg1, arg2, arg3, arg4, arg5,
                 SYSCALL_UNTRACED as *mut _, 0, 0)
}