mod util;
use util::Builder;

#[test]
fn build_default_project() {
    let _build = Builder::build("default").unwrap();
}

#[test]
fn build_example_project() {
    let _build = Builder::build("example").unwrap();
}
