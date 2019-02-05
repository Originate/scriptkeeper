extern crate cc;

fn main() {
    cc::Build::new()
        .file("src/c-tracing-poc.c")
        .compile("c-tracing-poc");
}
