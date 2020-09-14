extern crate thread_io;

use std::cmp::min;
use std::io::{self, Write};
use thread_io::write::*;

/// a writer that only writes its data to `Writer::data` upon `flush()`
#[derive(Clone)]
struct Writer {
    cache: Vec<u8>,
    data: Vec<u8>,
    write_fails: bool,
    flush_fails: bool,
    bufsize: usize,
}

impl Writer {
    fn new(write_fails: bool, flush_fails: bool, bufsize: usize) -> Writer {
        Writer {
            cache: vec![],
            data: vec![],
            write_fails: write_fails,
            flush_fails: flush_fails,
            bufsize: bufsize,
        }
    }

    fn data(&self) -> &[u8] {
        &self.data
    }
}

impl Write for Writer {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.write_fails {
            return Err(io::Error::new(io::ErrorKind::Other, "write err"));
        }
        self.cache.write(&buffer[..min(buffer.len(), self.bufsize)])
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.flush_fails {
            Err(io::Error::new(io::ErrorKind::Other, "flush err"))
        } else {
            self.data.extend_from_slice(&self.cache);
            self.cache.clear();
            Ok(())
        }
    }
}

#[test]
fn write_thread() {
    let text = b"The quick brown fox jumps over the lazy dog";
    let n = text.len() + 3;

    for channel_bufsize in (1..n).step_by(2) {
        for writer_bufsize in (1..n).step_by(2) {
            for queuelen in (1..n).step_by(2) {
                // Test the writer: write without flushing, which should result in empty output
                let w = writer_init_finish(
                    channel_bufsize,
                    queuelen,
                    || Ok(Writer::new(false, false, writer_bufsize)),
                    |w| w.write(text),
                    |w| w,
                )
                .unwrap()
                .1;
                assert_eq!(w.data(), b"");

                // Write with flushing: the output should be equal to the written data
                let mut w = writer_init_finish(
                    channel_bufsize,
                    queuelen,
                    || Ok(Writer::new(false, false, writer_bufsize)),
                    |w| w.write(text),
                    |mut w| {
                        w.flush().unwrap();
                        w
                    },
                )
                .unwrap()
                .1;
                if w.data() != &text[..] {
                    panic!(format!(
                        "write test failed: {:?} != {:?} at channel buffer size {}, writer bufsize {}, queue length {}",
                        String::from_utf8_lossy(w.data()), String::from_utf8_lossy(&text[..]),
                        channel_bufsize, writer_bufsize, queuelen
                    ));
                }

                w.flush().unwrap();
                if w.data() != &text[..] {
                    panic!(format!(
                        "write test failed: {:?} != {:?} at channel buffer size {}, writer bufsize {}, queue length {}",
                        String::from_utf8_lossy(w.data()), String::from_utf8_lossy(&text[..]),
                        channel_bufsize, writer_bufsize, queuelen
                    ));
                }
            }
        }
    }
}

#[test]
fn writer_init_fail() {
    let e = io::Error::new(io::ErrorKind::Other, "init err");
    let res = writer_init(
        5,
        2,
        || Err::<&mut [u8], _>(e),
        |writer| writer.write(b"let the cows come home"),
    );
    if let Err(e) = res {
        assert_eq!(&format!("{}", e), "init err");
    } else {
        panic!("init should fail");
    }
}

#[test]
#[should_panic(expected = "init panic")]
fn writer_init_panic() {
    writer_init::<&mut Vec<u8>, _, _, _, _>(
        5,
        2,
        || panic!("init panic"),
        |writer| writer.write(b"let the cows come home"),
    ).unwrap();
}


#[test]
fn write_fail() {
    let text = b"The quick brown fox jumps over the lazy dog";
    let n = text.len() + 3;

    for channel_bufsize in (1..n).step_by(2) {
        for writer_bufsize in (1..n).step_by(2) {
            for queuelen in (1..n).step_by(2) {
                let w = Writer::new(true, false, writer_bufsize);
                let res = writer(channel_bufsize, queuelen, w, |w| w.write_all(text));
                if let Err(e) = res {
                    assert_eq!(&format!("{}", e), "write err");
                } else {
                    panic!("write should fail");
                }

                let w = Writer::new(false, true, writer_bufsize);
                let res = writer(channel_bufsize, queuelen, w, |w| w.flush());
                if let Err(e) = res {
                    assert_eq!(&format!("{}", e), "flush err");
                } else {
                    panic!("flush should fail");
                }
            }
        }
    }
}

#[test]
fn write_source_fail() {
    let w = Writer::new(true, false, 1);
    let res: std::io::Result<()> = writer(1, 1, w, |_w| {
        Err(std::io::Error::from(std::io::ErrorKind::AddrInUse))
    });

    if let Err(e) = res {
        assert_eq!(e.kind(), std::io::ErrorKind::AddrInUse);
    } else {
        panic!("expected error")
    }
}

#[test]
#[should_panic(expected = "all out of bubblegum")]
fn write_source_panic() {
    let w = Writer::new(true, false, 1);
    let _res: std::io::Result<()> = writer(1, 1, w, |_w| panic!("all out of bubblegum"));
}
