mod util;
pub use util::*;

#[test]
fn keep_none() {
    let builder = ExeBuilder::init("keep-none")
        .unwrap()
        .run(&["make", "-v"])
        .unwrap();

    assert!(builder.out_dir().join("songbook.pdf").exists());
    assert!(!builder.out_dir().join("songbook.tex").exists());
    assert!(builder.find_tmp_dir("songbook.pdf").is_none());
}

#[test]
fn keep_tex_only() {
    let builder = ExeBuilder::init("keep-tex-only")
        .unwrap()
        .run(&["make", "-kv"])
        .unwrap();

    assert!(builder.out_dir().join("songbook.pdf").exists());
    assert!(builder.out_dir().join("songbook.tex").exists());
    assert!(builder.find_tmp_dir("songbook.pdf").is_none());
}

#[test]
fn keep_all() {
    let builder = ExeBuilder::init("keep-all")
        .unwrap()
        .run(&["make", "-kkv"])
        .unwrap();

    assert!(builder.out_dir().join("songbook.pdf").exists());
    assert!(builder.out_dir().join("songbook.tex").exists());
    let tmp_dir = builder.find_tmp_dir("songbook.pdf").unwrap();
    assert!(tmp_dir.join("songbook.toc").exists());
}
