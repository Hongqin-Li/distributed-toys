# MapReduce in Rust

This is a simple MapReduce implementation in Rust, following [6.824 Lab 1: MapReduce](https://pdos.csail.mit.edu/6.824/labs/lab-mr.html). Text corpus involved are borrowed directly from the lab.

Simply run `sh test.sh` to build and test the program, which will spawn a master and several workers in parallel, running the word count program in `src/app/wc.rs`, saving the intermediate json at `target/mr-*-*.json` and the result at `target/mr-out-*`. A sequential MapReduce (`src/bin/sequential.rs`) is also run to generate answer results for comparison with the output of parallel MapReduce.