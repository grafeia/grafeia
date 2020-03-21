#![feature(test)]

use grafeia_core::*;
use grafeia_core::draw::Cache;
use std::hint::black_box;


fn main() {
    use std::fs::File;
    env_logger::init();

    let mut args = std::env::args().skip(1);
    let input = File::open(args.next().expect("no input given")).expect("can't open input file");
    
    let state = State::load(input).unwrap();
    let mut cache = Cache::new();

    for i in 0 .. 1 {
        black_box(cache.layout(&state.storage, &state.design, &state.target, state.root));
    }
}
