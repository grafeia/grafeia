use grafeia_core::*;
use grafeia_convert::*;
use std::fs::{self, File};

fn main() {
    let mut args = std::env::args().skip(1);
    let input = File::open(args.next().expect("no input file given")).expect("can't open input file");
    let output = args.next().expect("no output file given");

    let state = State::load(input).unwrap();
    let data = export::docx::export_docx(&state);
    fs::write(output, data).expect("can't open output file");
}
