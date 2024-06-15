//@ignore-target-windows: No libc socketpair on Windows
// test_race depends on a deterministic schedule.
//@compile-flags: -Zmiri-preemption-rate=0
use std::thread;
fn main() {
    test_socketpair();
    test_socketpair_threaded();
    test_race();
}

fn test_socketpair() {
    let mut fds = [-1, -1];
    let mut res =
        unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };
    assert_eq!(res, 0);

    // Read size == data available in buffer.
    let data = "abcde".as_bytes().as_ptr();
    res = unsafe { libc::write(fds[0], data as *const libc::c_void, 5).try_into().unwrap() };
    assert_eq!(res, 5);
    let mut buf: [u8; 5] = [0; 5];
    res = unsafe {
        libc::read(fds[1], buf.as_mut_ptr().cast(), buf.len() as libc::size_t).try_into().unwrap()
    };
    assert_eq!(res, 5);
    assert_eq!(buf, "abcde".as_bytes());

    // Read size > data available in buffer.
    let data = "abc".as_bytes().as_ptr();
    res = unsafe { libc::write(fds[0], data as *const libc::c_void, 3).try_into().unwrap() };
    assert_eq!(res, 3);
    let mut buf2: [u8; 5] = [0; 5];
    res = unsafe {
        libc::read(fds[1], buf2.as_mut_ptr().cast(), buf2.len() as libc::size_t).try_into().unwrap()
    };
    assert_eq!(res, 3);
    assert_eq!(&buf2[0..3], "abc".as_bytes());

    // Test read and write from another direction.
    // Read size == data available in buffer.
    let data = "12345".as_bytes().as_ptr();
    res = unsafe { libc::write(fds[1], data as *const libc::c_void, 5).try_into().unwrap() };
    assert_eq!(res, 5);
    let mut buf3: [u8; 5] = [0; 5];
    res = unsafe {
        libc::read(fds[0], buf3.as_mut_ptr().cast(), buf3.len() as libc::size_t).try_into().unwrap()
    };
    assert_eq!(res, 5);
    assert_eq!(buf3, "12345".as_bytes());

    // Read size > data available in buffer.
    let data = "123".as_bytes().as_ptr();
    res = unsafe { libc::write(fds[1], data as *const libc::c_void, 3).try_into().unwrap() };
    assert_eq!(res, 3);
    let mut buf4: [u8; 5] = [0; 5];
    res = unsafe {
        libc::read(fds[0], buf4.as_mut_ptr().cast(), buf4.len() as libc::size_t).try_into().unwrap()
    };
    assert_eq!(res, 3);
    assert_eq!(&buf4[0..3], "123".as_bytes());
}

fn test_socketpair_threaded() {
    let mut fds = [-1, -1];
    let mut res =
        unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };
    assert_eq!(res, 0);

    let data = "abcde".as_bytes().as_ptr();
    res = unsafe { libc::write(fds[0], data as *const libc::c_void, 5).try_into().unwrap() };
    assert_eq!(res, 5);
    let thread1 = thread::spawn(move || {
        let mut buf: [u8; 5] = [0; 5];
        let res: i64 = unsafe {
            libc::read(fds[1], buf.as_mut_ptr().cast(), buf.len() as libc::size_t)
                .try_into()
                .unwrap()
        };
        assert_eq!(res, 5);
        assert_eq!(buf, "abcde".as_bytes());
    });
    thread1.join().unwrap();

    // Read and write from different direction
    let thread2 = thread::spawn(move || {
        let data = "12345".as_bytes().as_ptr();
        let res: i64 =
            unsafe { libc::write(fds[0], data as *const libc::c_void, 5).try_into().unwrap() };
        assert_eq!(res, 5);
    });
    thread2.join().unwrap();
    let mut buf: [u8; 5] = [0; 5];
    res = unsafe {
        libc::read(fds[1], buf.as_mut_ptr().cast(), buf.len() as libc::size_t).try_into().unwrap()
    };
    assert_eq!(res, 5);
    assert_eq!(buf, "12345".as_bytes());
}
fn test_race() {
    static mut VAL: u8 = 0;
    let mut fds = [-1, -1];
    let mut res =
        unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };
    assert_eq!(res, 0);
    let thread1 = thread::spawn(move || {
        let mut buf: [u8; 1] = [0; 1];
        // write() from the main thread will occur before the read() here
        // because preemption is disabled and the main thread yields after write().
        let res: i32 = unsafe {
            libc::read(fds[1], buf.as_mut_ptr().cast(), buf.len() as libc::size_t)
                .try_into()
                .unwrap()
        };
        assert_eq!(res, 1);
        assert_eq!(buf, "a".as_bytes());
        unsafe { assert_eq!(VAL, 1) };
    });
    unsafe { VAL = 1 };
    let data = "a".as_bytes().as_ptr();
    res = unsafe { libc::write(fds[0], data as *const libc::c_void, 1).try_into().unwrap() };
    assert_eq!(res, 1);
    thread::yield_now();
    thread1.join().unwrap();
}
