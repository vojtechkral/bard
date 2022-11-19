use super::*;

#[test]
fn hb_helper_math() {
    let hb = Handlebars::new().with_helper("math", MathHelper);
    let math = move |expr: &str| {
        hb.render_template(&format!("{{{{ math {} }}}}", expr), &0)
            .unwrap()
    };

    // Numbers vs strings
    assert_eq!(math(r#" 1 "+" 1 "#), "2");
    assert_eq!(math(r#" 1 "+" "1" "#), "2");
    assert_eq!(math(r#" "1" "+" 1 "#), "2");
    assert_eq!(math(r#" "1" "+" "1" "#), "2");

    // Ints vs floats
    assert_eq!(math(r#" 1 "+" 1.1 "#), "2.1");
    assert_eq!(math(r#" -1.5 "+" 2 "#), "0.5");

    // Other int ops
    assert_eq!(math(r#" 1 "-" 1 "#), "0");
    assert_eq!(math(r#" 2 "*" 2 "#), "4");
    assert_eq!(math(r#" 9 "//" 2  "#), "4");
    assert_eq!(math(r#" 10 "%" 3 "#), "1");
    assert_eq!(math(r#" 5 "&" 3 "#), "1");
    assert_eq!(math(r#" 5 "|" 3 "#), "7");
    assert_eq!(math(r#" 5 "^" 3 "#), "6");
    assert_eq!(math(r#"1 "<<" 3"#), "8");
    assert_eq!(math(r#"8 ">>" 2"#), "2");

    // Float ops
    assert_eq!(math(r#" -1.1 "+" 1.1 "#), "0.0");
    assert_eq!(math(r#"8.8 "-" 4.4"#), "4.4");
    assert_eq!(math(r#"15.0 "*" 0.5"#), "7.5");
    assert_eq!(math(r#"90.0 "/" 3.0"#), "30.0");
    assert_eq!(math(r#"11.5 "%" 2.0"#), "1.5");
}
