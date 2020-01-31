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

//! re-exported auxv defined in <sys/auxv.h>
//!
pub const AT_NULL: usize = 0;
pub const AT_IGNORE: usize = 1;
pub const AT_EXECFD: usize = 2;
pub const AT_PHDR: usize = 3;
pub const AT_PHENT: usize = 4;
pub const AT_PHNUM: usize = 5;
pub const AT_PAGESZ: usize = 6;
pub const AT_BASE: usize = 7;
pub const AT_FLAGS: usize = 8;
pub const AT_ENTRY: usize = 9;
pub const AT_NOTELF: usize = 10;
pub const AT_UID: usize = 11;
pub const AT_EUID: usize = 12;
pub const AT_GID: usize = 13;
pub const AT_EGID: usize = 14;
pub const AT_CLKTCK: usize = 17;
pub const AT_PLATFORM: usize = 15;
pub const AT_HWCAP: usize = 16;
pub const AT_FPUCW: usize = 18;
pub const AT_DCACHEBSIZE: usize = 19;
pub const AT_ICACHEBSIZE: usize = 20;
pub const AT_UCACHEBSIZE: usize = 21;
pub const AT_IGNOREPPC: usize = 22;
pub const AT_SECURE: usize = 23;
pub const AT_BASE_PLATFORM: usize = 24;
pub const AT_RANDOM: usize = 25;
pub const AT_HWCAP2: usize = 26;
pub const AT_EXECFN: usize = 31;
pub const AT_SYSINFO: usize = 32;
pub const AT_SYSINFO_EHDR: usize = 33;
pub const AT_L1I_CACHESHAPE: usize = 34;
pub const AT_L1D_CACHESHAPE: usize = 35;
pub const AT_L2_CACHESHAPE: usize = 36;
pub const AT_L3_CACHESHAPE: usize = 37;
pub const AT_L1I_CACHESIZE: usize = 40;
pub const AT_L1I_CACHEGEOMETRY: usize = 41;
pub const AT_L1D_CACHESIZE: usize = 42;
pub const AT_L1D_CACHEGEOMETRY: usize = 43;
pub const AT_L2_CACHESIZE: usize = 44;
pub const AT_L2_CACHEGEOMETRY: usize = 45;
pub const AT_L3_CACHESIZE: usize = 46;
pub const AT_L3_CACHEGEOMETRY: usize = 47;
