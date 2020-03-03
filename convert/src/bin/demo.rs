use grafeia_convert::build;
use std::fs::File;
fn main() {
    let out = File::create("demo.graf").unwrap();
    build::build().store(out).unwrap();
}