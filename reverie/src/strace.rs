#![allow(unused_imports)]
#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

use clap::{App, Arg};
use fern;
use libc;
use nix::fcntl::OFlag;
use nix::sys::stat::Mode;
use nix::sys::wait::WaitStatus;
use nix::sys::{memfd, mman, ptrace, signal, wait};
use nix::unistd;
use nix::unistd::ForkResult;
use std::collections::HashMap;
use std::env;
use std::ffi::CString;
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use reverie_api::event::*;
use reverie_api::remote::*;
use reverie_api::task::*;

use reverie::reverie_common::{consts, state::*};
use reverie::sched_wait::SchedWait;
use reverie::{hooks, ns};

use reverie_seccomp::seccomp_bpf;

#[test]
fn can_resolve_syscall_hooks() -> Result<()> {
    let so = PathBuf::from("../lib").join("libecho.so").canonicalize()?;
    let parsed = hooks::resolve_syscall_hooks_from(so)?;
    assert_ne!(parsed.len(), 0);
    Ok(())
}

struct Arguments<'a> {
    debug_level: i32,
    host_envs: bool,
    envs: HashMap<String, String>,
    namespaces: bool,
    output: Option<&'a str>,
    disable_monkey_patcher: bool,
    show_perf_stats: bool,
    program: &'a str,
    program_args: Vec<&'a str>,
}

fn run_tracer_main<G>(sched: &mut SchedWait<G>) -> i32 {
    sched.run_all()
}

fn wait_sigstop(pid: unistd::Pid) -> Result<()> {
    match wait::waitpid(Some(pid), None).expect("waitpid failed") {
        WaitStatus::Stopped(new_pid, signal)
            if signal == signal::SIGSTOP && new_pid == pid =>
        {
            Ok(())
        }
        _ => Err(Error::new(ErrorKind::Other, "expect SIGSTOP")),
    }
}

fn from_nix_error(err: nix::Error) -> Error {
    Error::new(ErrorKind::Other, err)
}

// hardcoded because `libc` does not export
const PER_LINUX: u64 = 0x0;
const ADDR_NO_RANDOMIZE: u64 = 0x0004_0000;

fn tracee_init_signals() {
    unsafe {
        let _ = signal::sigaction(
            signal::SIGTTIN,
            &signal::SigAction::new(
                signal::SigHandler::SigIgn,
                signal::SaFlags::SA_RESTART,
                signal::SigSet::empty(),
            ),
        );
        let _ = signal::sigaction(
            signal::SIGTTOU,
            &signal::SigAction::new(
                signal::SigHandler::SigIgn,
                signal::SaFlags::SA_RESTART,
                signal::SigSet::empty(),
            ),
        );
    };
}

fn run_tracee(argv: &Arguments) -> Result<i32> {
    unsafe {
        assert!(libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) == 0);
        assert!(libc::personality(PER_LINUX | ADDR_NO_RANDOMIZE) != -1);
    };

    ptrace::traceme()
        .and_then(|_| signal::raise(signal::SIGSTOP))
        .map_err(from_nix_error)?;

    tracee_init_signals();

    let mut envs: Vec<String> = Vec::new();

    if argv.host_envs {
        std::env::vars().for_each(|(k, v)| {
            envs.push(format!("{}={}", k, v));
        });
    } else {
        envs.push(String::from("PATH=/bin/:/usr/bin"));
    }

    argv.envs.iter().for_each(|(k, v)| {
        if v.is_empty() {
            envs.push(k.to_string())
        } else {
            envs.push(format!("{}={}", k, v));
        }
    });

    let program = CString::new(argv.program)?;
    let mut args: Vec<CString> = Vec::new();
    CString::new(argv.program).map(|s| args.push(s))?;
    for v in argv.program_args.clone() {
        CString::new(v).map(|s| args.push(s))?;
    }
    let envp: Vec<CString> = envs
        .into_iter()
        .map(|s| CString::new(s.as_bytes()).unwrap())
        .collect();

    log::info!(
        "[main] launching: {} {:?}",
        &argv.program,
        &argv.program_args
    );

    let mut whitelist: Vec<_> = vec![(0x7000_0002, 0x7000_0002)];
    let bytes = seccomp_bpf::bpf_whitelist_ips(whitelist.as_mut());
    let rr = seccomp_bpf::seccomp(&bytes);
    println!("seccomp returned: {:?}", rr);

    unistd::execvpe(&program, args.as_slice(), envp.as_slice())
        .map_err(from_nix_error)?;
    panic!("exec failed: {} {:?}", &argv.program, &argv.program_args);
}

fn show_perf_stats(state: &ReverieState) {
    log::info!("Reverie global statistics (tracer + tracees):");
    let lines: Vec<String> =
        format!("{:#?}", state).lines().map(String::from).collect();
    for s in lines.iter().take(lines.len() - 1).skip(1) {
        log::info!("{}", s);
    }

    let syscalls = state.stats.nr_syscalls.load(Ordering::SeqCst);
    let syscalls_ptraced =
        state.stats.nr_syscalls_ptraced.load(Ordering::SeqCst);
    let syscalls_captured =
        state.stats.nr_syscalls_captured.load(Ordering::SeqCst);
    let syscalls_patched =
        state.stats.nr_syscalls_patched.load(Ordering::SeqCst);

    log::info!(
        "syscalls ptraced (slow): {:.2}%",
        100.0 * syscalls_ptraced as f64 / syscalls as f64
    );
    log::info!(
        "syscalls captured(w/ patching): {:.2}%",
        100.0 * syscalls_captured as f64 / syscalls as f64
    );
    log::info!(
        "syscalls captured(wo/ patching): {:.2}%",
        100.0 * (syscalls_captured - syscalls_patched) as f64 / syscalls as f64
    );
}

fn task_exec_cb(task: &mut dyn Task) -> Result<()> {
    log::trace!("[pid {}] exec cb", task.gettid());
    if let Some(init_proc_state) =
        task.resolve_symbol_address("init_process_state")
    {
        let args = SyscallArgs::from(0, 0, 0, 0, 0, 0);
        task.inject_funcall(init_proc_state, &args);
    }
    Ok(())
}
fn task_fork_cb(task: &mut dyn Task) -> Result<()> {
    log::trace!("[pid {}] fork cb", task.gettid());
    if let Some(init_proc_state) =
        task.resolve_symbol_address("init_process_state")
    {
        let args = SyscallArgs::from(0, 0, 0, 0, 0, 0);
        task.inject_funcall(init_proc_state, &args);
    }
    Ok(())
}
fn task_clone_cb(task: &mut dyn Task) -> Result<()> {
    log::trace!("[pid {}] clone cb", task.gettid());
    Ok(())
}
fn task_exit_cb(_exit_code: i32) -> Result<()> {
    Ok(())
}

fn run_tracer(
    starting_pid: unistd::Pid,
    starting_uid: unistd::Uid,
    starting_gid: unistd::Gid,
    argv: &Arguments,
) -> Result<i32> {
    // tracer is the 1st process in the new namespace.
    if argv.namespaces {
        ns::init_ns(starting_pid, starting_uid, starting_gid)?;
        debug_assert!(unistd::getpid() == unistd::Pid::from_raw(1));
    }

    let memfd_name = std::ffi::CStr::from_bytes_with_nul(&[
        b'r', b'e', b'v', b'e', b'r', b'i', b'e', 0,
    ])
    .unwrap();
    let fd_ = memfd::memfd_create(&memfd_name, memfd::MemFdCreateFlag::empty())
        .expect("memfd_create failed");
    let memfd = unistd::dup2(fd_, consts::REVERIE_GLOBAL_STATE_FD)
        .expect("dup2 to REVERIE_GLOBAL_STATE_FD failed");
    let _ = unistd::close(fd_);
    let glob_size = 32768 * 4096;
    let _ = unistd::ftruncate(memfd, 32768 * 4096)
        .expect(&format!("memfd, unable to alloc {} bytes.", glob_size));

    match unistd::fork().expect("fork failed") {
        ForkResult::Child => run_tracee(argv),
        ForkResult::Parent { child } => {
            // wait for sigstop
            wait_sigstop(child)?;
            ptrace::setoptions(
                child,
                ptrace::Options::PTRACE_O_TRACEEXEC
                    | ptrace::Options::PTRACE_O_EXITKILL
                    | ptrace::Options::PTRACE_O_TRACECLONE
                    | ptrace::Options::PTRACE_O_TRACEFORK
                    | ptrace::Options::PTRACE_O_TRACEVFORK
                    | ptrace::Options::PTRACE_O_TRACEVFORKDONE
                    | ptrace::Options::PTRACE_O_TRACEEXIT
                    | ptrace::Options::PTRACE_O_TRACESECCOMP
                    | ptrace::Options::PTRACE_O_TRACESYSGOOD,
            )
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
            ptrace::cont(child, None)
                .map_err(|e| Error::new(ErrorKind::Other, e))?;
            let tracee = Task::new(child);
            let cbs = TaskEventCB::new(
                Box::new(task_exec_cb),
                Box::new(task_fork_cb),
                Box::new(task_clone_cb),
                Box::new(task_exit_cb),
            );
            let mut sched: SchedWait<i32> = SchedWait::new(cbs, 0);
            sched.add(tracee);
            let res = run_tracer_main(&mut sched);
            if argv.show_perf_stats {
                let _ = reverie_global_state().lock().as_ref().and_then(|st| {
                    show_perf_stats(st);
                    Ok(())
                });
            }
            Ok(res)
        }
    }
}

fn run_app(argv: &Arguments) -> Result<i32> {
    let (starting_pid, starting_uid, starting_gid) =
        (unistd::getpid(), unistd::getuid(), unistd::getgid());

    if argv.namespaces {
        unsafe {
            assert!(
                libc::unshare(
                    libc::CLONE_NEWUSER
                        | libc::CLONE_NEWPID
                        | libc::CLONE_NEWNS
                        | libc::CLONE_NEWUTS
                ) == 0
            );
        };

        match unistd::fork().expect("fork failed") {
            ForkResult::Child => {
                run_tracer(starting_pid, starting_uid, starting_gid, argv)
            }
            ForkResult::Parent { child } => {
                match wait::waitpid(Some(child), None) {
                    Ok(wait::WaitStatus::Exited(_, exit_code)) => Ok(exit_code),
                    Ok(wait::WaitStatus::Signaled(_, sig, _)) => {
                        Ok(0x80 | sig as i32)
                    }
                    otherwise => panic!(
                        "unexpected status from waitpid: {:?}",
                        otherwise
                    ),
                }
            }
        }
    } else {
        run_tracer(starting_pid, starting_uid, starting_gid, argv)
    }
}

fn populate_rpath(hint: Option<&str>, so: &str) -> Result<PathBuf> {
    let mut exe_path = env::current_exe()?;
    exe_path.pop();
    let search_path = vec![PathBuf::from("."), PathBuf::from("lib"), exe_path];
    let rpath = match hint {
        Some(path) => PathBuf::from(path).canonicalize().ok(),
        None => search_path
            .iter()
            .find(|p| match p.join(so).canonicalize() {
                Ok(fp) => fp.exists(),
                Err(_) => false,
            })
            .cloned(),
    };
    log::trace!("[main] library search path: {:?}", search_path);
    log::info!("[main] library-path chosen: {:?}", rpath);
    rpath.ok_or_else(|| {
        Error::new(ErrorKind::NotFound, "cannot find a valid library path")
    })
}

fn main() {
    let matches = App::new("reverie - a fast syscall tracer and interceper")
        .version("0.0.1")
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .value_name("DEBUG_LEVEL")
                .help("Set debug level [0..5]")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("no-host-envs")
                .long("no-host-envs")
                .help("do not pass-through host's environment variables")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("env")
                .long("env")
                .value_name("ENV=VALUE")
                .multiple(true)
                .help("set environment variables, can be used multiple times")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("with-namespace")
                .long("with-namespace")
                .help("enable namespaces, including PID, USER, MOUNT.. default is false")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("with-log")
                .long("with-log")
                .value_name("OUTPUT")
                .help("with-log=[filename|stdout|stderr], default is stdout")
                .takes_value(true),
        )
        .arg(Arg::with_name("disable-monkey-patcher")
             .long("disable-monkey-patcher")
             .help("do not patch any syscalls, handle all syscalls by seccomp")
             .takes_value(false)
        )
        .arg(Arg::with_name("show-perf-stats")
             .long("show-perf-stats")
             .help("show reverie softare performance counter statistics, --debug must be >= 3")
             .takes_value(false)
        )
        .arg(
            Arg::with_name("program")
                .value_name("PROGRAM")
                .required(true)
                .help("PROGRAM")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("program_args")
                .value_name("PROGRAM_ARGS")
                .allow_hyphen_values(true)
                .multiple(true)
                .help("[PROGRAM_ARGUMENTS..]"),
        )
        .get_matches();

    let log_level = matches
        .value_of("debug")
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(0);
    let log_output = matches.value_of("with-log");
    setup_logger(log_level, log_output).expect("set log level");

    let argv = Arguments {
        debug_level: log_level,
        host_envs: !matches.is_present("-no-host-envs"),
        envs: matches
            .values_of("env")
            .unwrap_or_default()
            .map(|s| {
                let t: Vec<&str> = s.split('=').collect();
                debug_assert!(!t.is_empty());
                (t[0].to_string(), t[1..].join("="))
            })
            .collect(),
        namespaces: matches.is_present("with-namespace"),
        output: log_output,
        disable_monkey_patcher: matches.is_present("disable-monkey-patcher"),
        show_perf_stats: matches.is_present("show-perf-stats"),
        program: matches.value_of("program").unwrap_or(""),
        program_args: matches
            .values_of("program_args")
            .map(|v| v.collect())
            .unwrap_or_else(Vec::new),
    };

    match run_app(&argv) {
        Ok(exit_code) => std::process::exit(exit_code),
        err => panic!("run app failed with error: {:?}", err),
    }
}

fn fern_with_output(output: Option<&str>) -> Result<fern::Dispatch> {
    match output {
        None => Ok(fern::Dispatch::new().chain(std::io::stdout())),
        Some(s) => match s {
            "stdout" => Ok(fern::Dispatch::new().chain(std::io::stdout())),
            "stderr" => Ok(fern::Dispatch::new().chain(std::io::stderr())),
            output => {
                let f = std::fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(output)?;
                Ok(fern::Dispatch::new().chain(f))
            }
        },
    }
}

fn setup_logger(level: i32, output: Option<&str>) -> Result<()> {
    let log_level = match level {
        0 => log::LevelFilter::Off,
        1 => log::LevelFilter::Error,
        2 => log::LevelFilter::Warn,
        3 => log::LevelFilter::Info,
        4 => log::LevelFilter::Debug,
        5 => log::LevelFilter::Trace,
        _ => log::LevelFilter::Trace,
    };

    fern_with_output(output)?
        .level(log_level)
        .format(|out, message, _record| out.finish(format_args!("{}", message)))
        .apply()
        .map_err(|e| Error::new(ErrorKind::Other, e))
}