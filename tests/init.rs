mod util;
pub use util::*;

#[test]
fn init_and_build() {
    let _build = Builder::init_and_build("init").unwrap();
}
