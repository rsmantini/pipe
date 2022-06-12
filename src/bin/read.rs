#![feature(io_slice_advance)]

use nix::errno::Errno;
use nix::fcntl::{self, SpliceFFlags};
use pipe::Mode;
use std::env;
use std::io::{self, prelude::*, BufReader, IoSlice};
use std::time::Instant;

const GB: usize = 1024 * 1024 * 1024;
const STDIN_FD: i32 = 0;

fn main() {
    let block_size: usize = env::args().nth(1).unwrap().parse().unwrap();
    let disable_memcpy: bool = env::args().nth(2).unwrap().parse().unwrap();
    let mode: Mode = env::args().nth(3).unwrap().parse().unwrap();
    let use_huge_pages: bool = env::args().nth(4).unwrap().parse().unwrap();
    println!("Start reading [disable memcpy = {}]", disable_memcpy);

    let start = Instant::now();

    let bytes_read = match mode {
        Mode::Pipe => run_pipe(block_size, disable_memcpy),
        Mode::Vmsplice => run_vmsplice(block_size, disable_memcpy, use_huge_pages),
    };

    let secs = start.elapsed().as_secs_f64();
    let gbs = bytes_read / GB;
    println!(
        "Read {} GB in {} seconds | {} GB/s",
        gbs,
        secs,
        gbs as f64 / secs
    );
    println!("    Exactly: {}", bytes_read);
}

fn run_pipe(block_size: usize, disable_memcpy: bool) -> usize {
    let mut reader = BufReader::new(io::stdin());
    let mut in_buf = vec![0; block_size];
    let mut _out_buf = vec![0; block_size];
    let mut bytes_read = 0;

    while reader.read_exact(&mut in_buf).is_ok() {
        if !disable_memcpy {
            _out_buf.copy_from_slice(&in_buf);
        }
        bytes_read += in_buf.len();
    }
    bytes_read
}

fn run_vmsplice(block_size: usize, disable_memcpy: bool, use_huge_pages: bool) -> usize {
    let in_buf = create_buffer(block_size, use_huge_pages);
    let mut _out_buf = vec![0; block_size];
    let mut bytes_read = 0;
    let mut done = false;
    while !done {
        let mut iov = [IoSlice::new(&in_buf)];
        let mut r = 0;
        while r < block_size {
            match fcntl::vmsplice(STDIN_FD, &iov, SpliceFFlags::SPLICE_F_NONBLOCK) {
                Ok(bytes) if bytes > 0 => {
                    iov[0].advance(bytes);
                    r += bytes;
                }
                Err(Errno::EAGAIN) => continue,
                _ => {
                    done = true;
                    break;
                }
            };
        }
        bytes_read += r;
        if !disable_memcpy {
            _out_buf.copy_from_slice(&in_buf);
        }
    }
    bytes_read
}

fn create_buffer(block_size: usize, use_huge_pages: bool) -> Vec<u8> {
    if use_huge_pages {
        let mut ptr = std::ptr::null_mut();
        unsafe {
            libc::posix_memalign(&mut ptr, 1 << 21, block_size);
            if ptr.is_null() {
                panic!("could not allocate");
            }
            libc::madvise(ptr, block_size, libc::MADV_HUGEPAGE);
            std::ptr::write_bytes(ptr, 0, block_size);
            Vec::from_raw_parts(ptr.cast(), block_size, block_size)
        }
    } else {
        vec![0; block_size]
    }
}
