// Thread-ID -- Get a unique thread ID
// Copyright 2016 Ruud van Asseldonk
//
// Licensed under either the Apache License, Version 2.0, or the MIT license, at
// your option. A copy of both licenses has been included in the root of the
// repository.

#![warn(missing_docs)]

#[cfg(unix)]
extern crate libc;

#[cfg(windows)]
extern crate windows_sys;

thread_local! {
    static THREAD_ID: usize = get_internal()
}

/// Returns a number that is unique to the calling thread.
///
/// Calling this function twice from the same thread will return the same
/// number. Calling this function from a different thread will return a
/// different number.
#[inline]
pub(crate) fn get() -> usize {
    THREAD_ID.with(|inner| *inner)
}

#[cfg(all(
    unix,
    not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "tvos",
        target_os = "watchos"
    ))
))]
#[inline]
fn get_internal() -> usize {
    unsafe { libc::pthread_self() as usize }
}

#[cfg(all(
    unix,
    any(
        target_os = "macos",
        target_os = "ios",
        target_os = "tvos",
        target_os = "watchos"
    )
))]
#[inline]
fn get_internal() -> usize {
    let mut tid: u64 = 0;
    unsafe {
        libc::pthread_threadid_np(libc::pthread_self(), &mut tid as *mut u64);
    };
    tid as usize
}

#[cfg(windows)]
#[inline]
fn get_internal() -> usize {
    unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() as usize }
}

#[cfg(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown",
    not(target_feature = "atomics"),
))]
#[inline]
fn get_internal() -> usize {
    0
}

#[cfg(any(
    target_env = "sgx",
    all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown",
        target_feature = "atomics",
    )
))]
#[inline]
fn get_internal() -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    thread_local! {
        static ID: usize = COUNTER.fetch_add(1, Ordering::Relaxed);
    }

    ID.with(|id| *id)
}

#[test]
fn distinct_threads_have_distinct_ids() {
    use std::sync::mpsc;
    use std::thread;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || tx.send(get()).unwrap())
        .join()
        .unwrap();

    let main_tid = get();
    let other_tid = rx.recv().unwrap();
    assert!(main_tid != other_tid);
}
