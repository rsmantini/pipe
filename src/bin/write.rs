#![feature(io_safety, io_slice_advance)]

use clap::Parser;
use nix::errno::Errno;
use nix::fcntl::{self, SpliceFFlags};
use pipe::Mode;
use std::io::{BufWriter, IoSlice, Write};
use std::os::unix::prelude::AsRawFd;
use std::process::{Child, Command, Stdio};

const GB: usize = 1024 * 1024 * 1024;

fn main() {
    let args = Args::parse();
    println!(
        "Start writing [mode = {:?}, block-size = {}, use-huge-pages = {}]",
        args.mode, args.block_size, args.use_huge_pages
    );
    if let Ok(child) = Command::new("target/release/read")
        .args([
            args.block_size.to_string(),
            args.disable_read_memcpy.to_string(),
            args.mode.to_string(),
            args.use_huge_pages.to_string(),
        ])
        .stdin(Stdio::piped())
        .spawn()
    {
        match &args.mode {
            Mode::Pipe => run_pipe(&args, child),
            Mode::Vmsplice => run_vmsplice(&args, child),
        }
    }
}

fn run_pipe(args: &Args, mut child: Child) {
    let stdin = child.stdin.take().unwrap();
    let mut writer = BufWriter::new(stdin);
    let mut written = 0;
    let mut buf = vec![0; args.block_size];
    fill(&mut buf);
    while written < args.gbs * GB {
        writer.write_all(&buf).unwrap();
        written += buf.len();
    }
}

fn run_vmsplice(args: &Args, mut child: Child) {
    let stdin = child.stdin.take().unwrap();
    let buffer = create_buffer(args.block_size, args.use_huge_pages);
    let bufs = [&buffer[..args.block_size], &buffer[args.block_size..]];
    let mut buf_idx = 0;
    let mut written = 0;
    while written < args.gbs * GB {
        let mut iov = [IoSlice::new(bufs[buf_idx])];
        buf_idx = (buf_idx + 1) % 2;
        let mut w = 0;
        while w < args.block_size {
            match fcntl::vmsplice(stdin.as_raw_fd(), &iov, SpliceFFlags::SPLICE_F_NONBLOCK) {
                Ok(bytes) => {
                    iov[0].advance(bytes);
                    w += bytes;
                }
                Err(Errno::EAGAIN) => continue,
                _ => panic!(),
            }
        }
        written += w;
    }
}

fn create_buffer(block_size: usize, use_huge_pages: bool) -> Vec<u8> {
    let mut buf = if use_huge_pages {
        let mut ptr = std::ptr::null_mut();
        unsafe {
            libc::posix_memalign(&mut ptr, 1 << 21, 2 * block_size);
            if ptr.is_null() {
                panic!("could not allocate");
            }
            libc::madvise(ptr, 2 * block_size, libc::MADV_HUGEPAGE);
            Vec::from_raw_parts(ptr.cast(), 2 * block_size, 2 * block_size)
        }
    } else {
        vec![0; 2 * block_size]
    };
    fill(&mut buf);
    buf
}

fn fill(buf: &mut [u8]) {
    buf.iter_mut().enumerate().for_each(|(i, v)| {
        *v = if i % 2 == 0 { b'X' } else { b'\n' };
    });
}

/// Pair of writer/reader programs to test IPC speed
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Total number of gigabytes transfered
    #[clap(short, long)]
    gbs: usize,

    /// Number of bytes transfered per time
    #[clap(short, long, default_value_t = 65536)]
    block_size: usize,

    /// Disable copying the received data on the reader
    #[clap(short, long)]
    disable_read_memcpy: bool,

    /// Mechanism used to transfer data between processes
    #[clap(arg_enum, short, long, default_value_t = Mode::Pipe)]
    mode: Mode,

    /// Enable hugepages optimization
    #[clap(short, long)]
    use_huge_pages: bool,
}
