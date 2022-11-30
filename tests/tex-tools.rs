mod util;
pub use util::*;

#[test]
fn tex_tools_default_xelatex() {
    // Use xelatex when both xelatex and tectonic are available
    let builder = ExeBuilder::init("tex-tools-default-xelatex")
        .unwrap()
        .with_xelatex_bin()
        .with_tectonic_bin()
        .run(&["make", "-kv"])
        .unwrap();

    assert_first_line_contains(builder.out_dir().join("songbook.pdf"), "xelatex");
}

#[test]
fn tex_tools_tectonic() {
    // Use tectonic when it's the only one available
    let builder = ExeBuilder::init("tex-tools-tectonic")
        .unwrap()
        .with_tectonic_bin()
        .run(&["make", "-kv"])
        .unwrap();

    assert_first_line_contains(builder.out_dir().join("songbook.pdf"), "tectonic");
}

#[test]
fn tex_tools_tectonic_via_env() {
    let builder = ExeBuilder::init("tex-tools-tectonic-via-env")
        .unwrap()
        .with_xelatex_bin()
        .with_tectonic_bin()
        .with_env("BARD_TEX", "tectonic")
        .run(&["make", "-kv"])
        .unwrap();

    assert_first_line_contains(builder.out_dir().join("songbook.pdf"), "tectonic");
}

#[test]
fn tex_tools_env_full_path() {
    let tex_mock_exe = ExeBuilder::tex_mock_exe();
    let builder = ExeBuilder::init("tex-tools-env-full-path")
        .unwrap()
        .with_env("BARD_TEX", format!("texlive:{}", tex_mock_exe))
        .run(&["make", "-kv"])
        .unwrap();

    assert_first_line_contains(
        builder.out_dir().join("songbook.pdf"),
        tex_mock_exe.as_str(),
    );
}

#[test]
fn tex_tools_none() {
    let builder = ExeBuilder::init("tex-tools-none")
        .unwrap()
        .with_xelatex_bin()
        .with_tectonic_bin()
        .with_env("BARD_TEX", "none")
        .run(&["make", "-kv"])
        .unwrap();

    assert!(builder.out_dir().join("songbook.tex").exists());
}
