use consair::*;
use interner::InternedSymbol;
use language::{AtomType, StringType, SymbolType};

#[test]
fn test_basic_string() {
    let result = parse(r#""hello world""#).unwrap();
    match result {
        Value::Atom(AtomType::String(StringType::Basic(s))) => {
            assert_eq!(s, "hello world");
        }
        _ => panic!("Expected basic string, got {result:?}"),
    }
}

#[test]
fn test_string_with_escapes() {
    let result = parse(r#""hello\nworld""#).unwrap();
    match result {
        Value::Atom(AtomType::String(StringType::Basic(s))) => {
            assert_eq!(s, "hello\nworld");
        }
        _ => panic!("Expected basic string with newline, got {result:?}"),
    }
}

#[test]
fn test_string_with_unicode_escape() {
    let result = parse(r#""hello \u{1F600} world""#).unwrap();
    match result {
        Value::Atom(AtomType::String(StringType::Basic(s))) => {
            assert_eq!(s, "hello 😀 world");
        }
        _ => panic!("Expected string with emoji, got {result:?}"),
    }
}

// Raw string syntax has been removed in favor of maximum minimalism
// Use basic strings with escape sequences instead
// #[test]
// fn test_raw_string() {
//     let result = parse(r##"#"C:\path\to\file""##).unwrap();
//     match result {
//         Value::Atom(AtomType::String(StringType::Raw {
//             content,
//             hash_count,
//         })) => {
//             assert_eq!(content, r"C:\path\to\file");
//             assert_eq!(hash_count, 0);
//         }
//         _ => panic!("Expected raw string, got {result:?}"),
//     }
// }

// #[test]
// fn test_raw_string_with_hashes() {
//     let result = parse(r###"##"string with # in it"##"###).unwrap();
//     match result {
//         Value::Atom(AtomType::String(StringType::Raw {
//             content,
//             hash_count,
//         })) => {
//             assert_eq!(content, "string with # in it");
//             assert_eq!(hash_count, 2);
//         }
//         _ => panic!("Expected raw string with hashes, got {result:?}"),
//     }
// }

// Multiline string syntax has been removed
// #[test]
// fn test_multiline_string() {
//     let result = parse(
//         r#""""line1
// line2
// line3""""#,
//     )
//     .unwrap();
//     match result {
//         Value::Atom(AtomType::String(StringType::Multiline {
//             content,
//             interpolated,
//         })) => {
//             assert_eq!(content, "line1\nline2\nline3");
//             assert!(!interpolated);
//         }
//         _ => panic!("Expected multiline string, got {result:?}"),
//     }
// }

// Character literal syntax has been removed in favor of maximum minimalism
// Use single-character strings instead: "a" instead of #\a
// #[test]
// fn test_character_literal() {
//     let result = parse(r"#\a").unwrap();
//     match result {
//         Value::Atom(AtomType::Char('a')) => {}
//         _ => panic!("Expected character 'a', got {result:?}"),
//     }
// }

// Character literal syntax has been removed
// #[test]
// fn test_character_named() {
//     let result = parse(r"#\newline").unwrap();
//     match result {
//         Value::Atom(AtomType::Char('\n')) => {}
//         _ => panic!("Expected newline character, got {result:?}"),
//     }
// }

// Keyword syntax has been removed in favor of maximum minimalism
// #[test]
// fn test_simple_keyword() {
//     let result = parse(":name").unwrap();
//     match result {
//         Value::Atom(AtomType::Symbol(SymbolType::Keyword { name, namespace })) => {
//             assert_eq!(name, InternedSymbol::new("name"));
//             assert_eq!(namespace, None);
//         }
//         _ => panic!("Expected keyword :name, got {result:?}"),
//     }
// }

// #[test]
// fn test_namespaced_keyword() {
//     let result = parse(":user/name").unwrap();
//     match result {
//         Value::Atom(AtomType::Symbol(SymbolType::Keyword { name, namespace })) => {
//             assert_eq!(name, InternedSymbol::new("name"));
//             assert_eq!(namespace, Some(InternedSymbol::new("user")));
//         }
//         _ => panic!("Expected keyword :user/name, got {result:?}"),
//     }
// }

// #[test]
// fn test_auto_namespaced_keyword() {
//     let result = parse("::name").unwrap();
//     match result {
//         Value::Atom(AtomType::Symbol(SymbolType::Keyword { name, namespace })) => {
//             assert_eq!(name, InternedSymbol::new("name"));
//             assert_eq!(namespace, Some(InternedSymbol::new("__AUTO__")));
//         }
//         _ => panic!("Expected auto-namespaced keyword, got {result:?}"),
//     }
// }

// Byte string syntax has been removed in favor of maximum minimalism
// #[test]
// fn test_byte_string_ascii() {
//     let result = parse(r#"#b"hello""#).unwrap();
//     match result {
//         Value::Atom(AtomType::String(StringType::Bytes(bytes))) => {
//             assert_eq!(bytes, vec![b'h', b'e', b'l', b'l', b'o']);
//         }
//         _ => panic!("Expected byte string, got {result:?}"),
//     }
// }

// #[test]
// fn test_byte_string_hex() {
//     let result = parse(r#"#b[0xFF 0x00 0x11]"#).unwrap();
//     match result {
//         Value::Atom(AtomType::String(StringType::Bytes(bytes))) => {
//             assert_eq!(bytes, vec![0xFF, 0x00, 0x11]);
//         }
//         _ => panic!("Expected byte string with hex values, got {result:?}"),
//     }
// }

// Regex literal syntax has been removed in favor of maximum minimalism
// #[test]
// fn test_regex() {
//     let result = parse(r"~r/\d+/").unwrap();
//     match result {
//         Value::Atom(AtomType::String(StringType::Regex(re))) => {
//             assert!(re.is_match("123"));
//             assert!(!re.is_match("abc"));
//         }
//         _ => panic!("Expected regex, got {result:?}"),
//     }
// }

// #[test]
// fn test_regex_with_flags() {
//     let result = parse(r"~r/hello/i").unwrap();
//     match result {
//         Value::Atom(AtomType::String(StringType::Regex(re))) => {
//             assert!(re.is_match("HELLO"));
//             assert!(re.is_match("hello"));
//             assert!(re.is_match("HeLLo"));
//         }
//         _ => panic!("Expected case-insensitive regex, got {result:?}"),
//     }
// }

// Interpolated string syntax has been removed in favor of maximum minimalism
// #[test]
// fn test_interpolated_string_literal_only() {
//     let result = parse(r#"$"hello world""#).unwrap();
//     match result {
//         Value::Atom(AtomType::String(StringType::Interpolated { parts, is_raw })) => {
//             assert!(!is_raw);
//             assert_eq!(parts.len(), 1);
//             match &parts[0] {
//                 language::StringPart::Literal(s) => assert_eq!(s, "hello world"),
//                 _ => panic!("Expected literal part"),
//             }
//         }
//         _ => panic!("Expected interpolated string, got {result:?}"),
//     }
// }

// #[test]
// fn test_interpolated_string_with_expression() {
//     let result = parse(r#"$"Hello {name}!""#).unwrap();
//     match result {
//         Value::Atom(AtomType::String(StringType::Interpolated { parts, is_raw })) => {
//             assert!(!is_raw);
//             assert_eq!(parts.len(), 3);

//             match &parts[0] {
//                 language::StringPart::Literal(s) => assert_eq!(s, "Hello "),
//                 _ => panic!("Expected literal 'Hello '"),
//             }

//             match &parts[1] {
//                 language::StringPart::Expression(expr) => match expr.as_ref() {
//                     Value::Atom(AtomType::Symbol(SymbolType::Symbol(s))) => {
//                         assert_eq!(s, &InternedSymbol::new("name"));
//                     }
//                     _ => panic!("Expected symbol 'name'"),
//                 },
//                 _ => panic!("Expected expression"),
//             }

//             match &parts[2] {
//                 language::StringPart::Literal(s) => assert_eq!(s, "!"),
//                 _ => panic!("Expected literal '!'"),
//             }
//         }
//         _ => panic!("Expected interpolated string, got {result:?}"),
//     }
// }

// #[test]
// fn test_interpolated_string_with_lisp_expr() {
//     let result = parse(r#"$"Result: {(+ 1 2)}""#).unwrap();
//     match result {
//         Value::Atom(AtomType::String(StringType::Interpolated { parts, .. })) => {
//             assert_eq!(parts.len(), 2);

//             match &parts[1] {
//                 language::StringPart::Expression(expr) => {
//                     // Should be a list (+ 1 2)
//                     match expr.as_ref() {
//                         Value::Cons(_) => {
//                             // Validate it's a proper s-expression
//                             let result_str = format!("{expr}");
//                             assert_eq!(result_str, "(+ 1 2)");
//                         }
//                         _ => panic!("Expected cons cell for expression"),
//                     }
//                 }
//                 _ => panic!("Expected expression part"),
//             }
//         }
//         _ => panic!("Expected interpolated string, got {result:?}"),
//     }
// }

// Keyword syntax has been removed
// #[test]
// fn test_keywords_are_self_evaluating() {
//     let mut env = Environment::new();
//     let result = eval(parse(":test").unwrap(), &mut env).unwrap();
//
//     match result {
//         Value::Atom(AtomType::Symbol(SymbolType::Keyword { name, namespace })) => {
//             assert_eq!(name, InternedSymbol::new("test"));
//             assert_eq!(namespace, None);
//         }
//         _ => panic!("Expected keyword to be self-evaluating, got {result:?}"),
//     }
// }

#[test]
fn test_strings_are_self_evaluating() {
    let mut env = Environment::new();
    let result = eval(parse(r#""test""#).unwrap(), &mut env).unwrap();

    match result {
        Value::Atom(AtomType::String(StringType::Basic(s))) => {
            assert_eq!(s, "test");
        }
        _ => panic!("Expected string to be self-evaluating, got {result:?}"),
    }
}

// Character literal syntax has been removed
// #[test]
// fn test_chars_are_self_evaluating() {
//     let mut env = Environment::new();
//     let result = eval(parse(r"#\a").unwrap(), &mut env).unwrap();
//
//     match result {
//         Value::Atom(AtomType::Char('a')) => {}
//         _ => panic!("Expected char to be self-evaluating, got {result:?}"),
//     }
// }
