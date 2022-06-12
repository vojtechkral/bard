use std::fs;

use bard::util_cmd;

mod util;
pub use util::*;

#[test]
fn sort_lines() {
    let file = int_dir().join("test-file-sort-lines");
    let content_to_sort = r#"xxx
foo bar baz=b
foo bar baz=a
foo bar baz=d
foo bar baz=č

xxx

foo bar baz=b
foo bar baz=a
foo bar baz=c
xxx
"#;

    let expected = r#"xxx
foo bar baz=a
foo bar baz=b
foo bar baz=č
foo bar baz=d

xxx

foo bar baz=a
foo bar baz=b
foo bar baz=c
xxx
"#;

    fs::write(&file, content_to_sort.as_bytes()).unwrap();

    let count = util_cmd::sort_lines(r#"baz=(.+)$"#, file.as_str()).unwrap();
    let sorted_content = fs::read_to_string(&file).unwrap();

    assert_eq!(sorted_content, expected);
    assert_eq!(count, 7);
}

#[test]
fn sort_lines_no_capture_group() {
    let file = int_dir().join("test-file-sort-lines-no-capture-group");
    let content_to_sort = "foo bar baz=b\n";

    fs::write(&file, content_to_sort.as_bytes()).unwrap();
    util_cmd::sort_lines(r#"baz=.+$"#, file.as_str()).unwrap_err();
}

#[test]
fn sort_lines_no_match() {
    let file = int_dir().join("test-file-sort-lines-no-match");
    let content_to_sort = r#"xxx
yyy
zzz
"#;

    fs::write(&file, content_to_sort.as_bytes()).unwrap();
    let count = util_cmd::sort_lines(r#"baz=(.+)$"#, file.as_str()).unwrap();
    assert_eq!(count, 0);
}
