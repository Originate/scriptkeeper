extern crate cc;

fn main() {
    cc::Build::new().file("src/poc.c").compile("poc");
}
