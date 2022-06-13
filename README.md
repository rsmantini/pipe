# pipe
Experiment to measure Linux IPC speed

Implements all the optimizations described in: https://mazzo.li/posts/fast-pipes.html

The only difference is that the reader process does not use `splice` so it can actually read the data it receives

Also inspired by: https://github.com/brenoguim/shm

## Usage
**Compliles only with the nightly toolchain**
```
cargo build --release
cargo run --release --bin write -- -g 10 --mode vmsplice --use-huge-pages --disable-read-memcpy
```
## Command line options
```
USAGE:
    write [OPTIONS] --gbs <GBS>

OPTIONS:
    -b, --block-size <BLOCK_SIZE>    Number of bytes transfered per time [default: 65536]
    -d, --disable-read-memcpy        Disable copying the received data on the reader
    -g, --gbs <GBS>                  Total number of gigabytes transfered
    -h, --help                       Print help information
    -m, --mode <MODE>                Mechanism used to transfer data between processes [default:
                                     pipe] [possible values: pipe, vmsplice]
    -u, --use-huge-pages             Enable hugepages optimization
    -V, --version                    Print version information
```
