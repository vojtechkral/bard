use std::fs;

use bard::util::sort_lexical;
use bard::SortLinesOpts;

mod util;
pub use util::*;

#[test]
fn sort_lines() {
    let file = int_dir().join("lines-sort-test-file");
    let content_to_sort = r#"foo bar baz=b
foo bar baz=a
foo bar baz=d
foo bar baz=č
"#;

    fs::write(&file, content_to_sort.as_bytes()).unwrap();

    let sort_opts = SortLinesOpts {
        regex: r#"baz=(.+)$"#.to_owned(),
        file: file.to_str().unwrap().to_owned(),
    };
    bard::bard_sort_lines(&sort_opts).unwrap();

    let sorted_content = fs::read_to_string(&file).unwrap();
    let sorted_content: Vec<_> = sorted_content.lines().collect();
    let mut expected: Vec<_> = content_to_sort.lines().collect();
    sort_lexical(&mut expected);

    assert_eq!(sorted_content, expected);
}
