extern crate tempdir;

use super::{Token, Error, ErrorKind, Justfile};

use super::TokenKind::*;

fn tokenize_success(text: &str, expected_summary: &str) {
  let tokens = super::tokenize(text).unwrap();
  let roundtrip = tokens.iter().map(|t| {
    let mut s = String::new();
    s += t.prefix;
    s += t.lexeme;
    s
  }).collect::<Vec<_>>().join("");
  assert_eq!(text, roundtrip);
  assert_eq!(token_summary(&tokens), expected_summary);
}

fn tokenize_error(text: &str, expected: Error) {
  if let Err(error) = super::tokenize(text) {
    assert_eq!(error.text,   expected.text);
    assert_eq!(error.index,  expected.index);
    assert_eq!(error.line,   expected.line);
    assert_eq!(error.column, expected.column);
    assert_eq!(error.kind,   expected.kind);
    assert_eq!(error,        expected);
  } else {
    panic!("tokenize() succeeded but expected: {}\n{}", expected, text);
  }
}

fn token_summary(tokens: &[Token]) -> String {
  tokens.iter().map(|t| {
    match t.class {
      super::TokenKind::Line{..}    => "*",
      super::TokenKind::Name        => "N",
      super::TokenKind::Colon       => ":",
      super::TokenKind::Equals      => "=",
      super::TokenKind::Comment{..} => "#",
      super::TokenKind::Indent{..}  => ">",
      super::TokenKind::Dedent      => "<",
      super::TokenKind::Eol         => "$",
      super::TokenKind::Eof         => ".",
    }
  }).collect::<Vec<_>>().join("")
}

fn parse_success(text: &str) -> Justfile {
  match super::parse(text) {
    Ok(justfile) => justfile,
    Err(error) => panic!("Expected successful parse but got error:\n{}", error),
  }
}

fn parse_summary(input: &str, output: &str) {
  let justfile = parse_success(input);
  let mut s = String::new();
  for recipe in justfile.recipes {
    s += &format!("{}\n", recipe.1);
  }
  assert_eq!(s, output);
}

fn parse_error(text: &str, expected: Error) {
  if let Err(error) = super::parse(text) {
    assert_eq!(error.text,   expected.text);
    assert_eq!(error.index,  expected.index);
    assert_eq!(error.line,   expected.line);
    assert_eq!(error.column, expected.column);
    assert_eq!(error.kind,   expected.kind);
    assert_eq!(error.width,  expected.width);
    assert_eq!(error,        expected);
  } else {
    panic!("Expected {:?} but parse succeeded", expected.kind);
  }
}

#[test]
fn tokenize() {
  let text = "bob

hello blah blah blah : a b c #whatever
";
  tokenize_success(text, "N$$NNNN:NNN#$.");

  let text = "
hello:
  a
  b

  c

  d

bob:
  frank
  ";
  
  tokenize_success(text, "$N:$>*$*$$*$$*$$<N:$>*$<.");

  tokenize_success("a:=#", "N:=#.")
}

#[test]
fn inconsistent_leading_whitespace() {
  let text = "a:
 0
 1
\t2
";
  tokenize_error(text, Error {
    text:   text,
    index:  9,
    line:   3,
    column: 0,
    width:  None,
    kind:   ErrorKind::InconsistentLeadingWhitespace{expected: " ", found: "\t"},
  });

  let text = "a:
\t\t0
\t\t 1
\t  2
";
  tokenize_error(text, Error {
    text:   text,
    index:  12,
    line:   3,
    column: 0,
    width:  None,
    kind:   ErrorKind::InconsistentLeadingWhitespace{expected: "\t\t", found: "\t  "},
  });
}

#[test]
fn outer_shebang() {
  let text = "#!/usr/bin/env bash";
  tokenize_error(text, Error {
    text:   text,
    index:  0,
    line:   0,
    column: 0,
    width:  None,
    kind:   ErrorKind::OuterShebang
  });
}

#[test]
fn unknown_start_of_token() {
  let text = "~";
  tokenize_error(text, Error {
    text:   text,
    index:  0,
    line:   0,
    column: 0,
    width:  None,
    kind:   ErrorKind::UnknownStartOfToken
  });
}

#[test]
fn parse() {
  parse_summary("

# hello


  ", "");

  parse_summary("
x:
y:
z:
hello a b    c   : x y    z #hello
  #! blah
  #blarg
  1
  2
  3
", "hello a b c: x y z
    #! blah
    #blarg
    1
    2
    3
x:
y:
z:
");
}


#[test]
fn assignment_unimplemented() {
  let text = "a = z";
  parse_error(text, Error {
    text:   text,
    index:  2,
    line:   0,
    column: 2,
    width:  Some(1),
    kind:   ErrorKind::AssignmentUnimplemented
  });
}

#[test]
fn missing_colon() {
  let text = "a b c\nd e f";
  parse_error(text, Error {
    text:   text,
    index:  5,
    line:   0,
    column: 5,
    width:  Some(1),
    kind:   ErrorKind::UnexpectedToken{expected: vec![Name, Colon], found: Eol},
  });
}

#[test]
fn missing_eol() {
  let text = "a b c: z =";
  parse_error(text, Error {
    text:   text,
    index:  9,
    line:   0,
    column: 9,
    width:  Some(1),
    kind:   ErrorKind::UnexpectedToken{expected: vec![Name, Eol, Eof], found: Equals},
  });
}

#[test]
fn eof_test() {
  parse_summary("x:\ny:\nz:\na b c: x y z", "a b c: x y z\nx:\ny:\nz:\n");
}

#[test]
fn duplicate_argument() {
  let text = "a b b:";
  parse_error(text, Error {
    text:   text,
    index:  4,
    line:   0,
    column: 4,
    width:  Some(1),
    kind:   ErrorKind::DuplicateArgument{recipe: "a", argument: "b"}
  });
}

#[test]
fn duplicate_dependency() {
  let text = "a b c: b c z z";
  parse_error(text, Error {
    text:   text,
    index:  13,
    line:   0,
    column: 13,
    width:  Some(1),
    kind:   ErrorKind::DuplicateDependency{recipe: "a", dependency: "z"}
  });
}

#[test]
fn duplicate_recipe() {
  let text = "a:\nb:\na:";
  parse_error(text, Error {
    text:   text,
    index:  6,
    line:   2,
    column: 0,
    width:  Some(1),
    kind:   ErrorKind::DuplicateRecipe{recipe: "a", first: 0}
  });
}

#[test]
fn circular_dependency() {
  let text = "a: b\nb: a";
  parse_error(text, Error {
    text:   text,
    index:  8,
    line:   1,
    column: 3,
    width:  Some(1),
    kind:   ErrorKind::CircularDependency{recipe: "b", circle: vec!["a", "b", "a"]}
  });
}

#[test]
fn unknown_dependency() {
  let text = "a: b";
  parse_error(text, Error {
    text:   text,
    index:  3,
    line:   0,
    column: 3,
    width:  Some(1),
    kind:   ErrorKind::UnknownDependency{recipe: "a", unknown: "b"}
  });
}

#[test]
fn mixed_leading_whitespace() {
  let text = "a:\n\t echo hello";
  parse_error(text, Error {
    text:   text,
    index:  3,
    line:   1,
    column: 0,
    width:  None,
    kind:   ErrorKind::MixedLeadingWhitespace{whitespace: "\t "}
  });
}

#[test]
fn write_or() {
  assert_eq!("1",             super::Or(&[1      ]).to_string());
  assert_eq!("1 or 2",        super::Or(&[1,2    ]).to_string());
  assert_eq!("1, 2, or 3",    super::Or(&[1,2,3  ]).to_string());
  assert_eq!("1, 2, 3, or 4", super::Or(&[1,2,3,4]).to_string());
}

#[test]
fn run_shebang() {
  // this test exists to make sure that shebang recipes
  // run correctly. although this script is still
  // executed by sh its behavior depends on the value of a
  // variable and continuing even though a command fails,
  // whereas in plain recipes variables are not available
  // in subsequent lines and execution stops when a line
  // fails
  let text = "
a:
 #!/usr/bin/env sh
 code=200
 function x { return $code; }
 x
 x
";

  match parse_success(text).run(&["a"]).unwrap_err() {
    super::RunError::Code{recipe, code} => {
      assert_eq!(recipe, "a");
      assert_eq!(code, 200);
    },
    other @ _ => panic!("expected an code run error, but got: {}", other),
  }
}

#[test]
fn run_order() {
  let tmp = tempdir::TempDir::new("run_order").unwrap_or_else(|err| panic!("tmpdir: failed to create temporary directory: {}", err));
  let path = tmp.path().to_str().unwrap_or_else(|| panic!("tmpdir: path was not valid UTF-8")).to_owned();
  let text = r"
b: a
  @mv a b

a:
  @touch a

d: c
  @rm c

c: b
  @mv b c
";
  super::std::env::set_current_dir(path).expect("failed to set current directory");
  parse_success(text).run(&["a", "d"]).unwrap();
}

#[test]
fn unknown_recipes() {
  match parse_success("a:\nb:\nc:").run(&["a", "x", "y", "z"]).unwrap_err() {
    super::RunError::UnknownRecipes{recipes} => assert_eq!(recipes, &["x", "y", "z"]),
    other @ _ => panic!("expected an unknown recipe error, but got: {}", other),
  }
}

#[test]
fn code_error() {
  match parse_success("fail:\n @function x { return 100; }; x").run(&["fail"]).unwrap_err() {
    super::RunError::Code{recipe, code} => {
      assert_eq!(recipe, "fail");
      assert_eq!(code, 100);
    },
    other @ _ => panic!("expected a code run error, but got: {}", other),
  }
}

#[test]
fn extra_whitespace() {
  // we might want to make extra leading whitespace a line continuation in the future,
  // so make it a error for now
  let text = "a:\n blah\n  blarg";
  parse_error(text, Error {
    text:   text,
    index:  10,
    line:   2,
    column: 1,
    width:  Some(6),
    kind:   ErrorKind::ExtraLeadingWhitespace
  });

  // extra leading whitespace is okay in a shebang recipe
  parse_success("a:\n #!\n  print(1)");
}

#[test]
fn bad_recipe_names() {
  // We are extra strict with names. Although the tokenizer
  // will tokenize anything that matches /[a-zA-Z0-9_-]+/
  // as a name, we throw an error if names do not match
  // /[a-z](-?[a-z])*/. This is to support future expansion
  // of justfile and command line syntax.
  fn bad_name(text: &str, name: &str, index: usize, line: usize, column: usize) {
    parse_error(text, Error {
      text:   text,
      index:  index,
      line:   line,
      column: column,
      width:  Some(name.len()),
      kind:   ErrorKind::BadName{name: name}
    });
  }

  bad_name("-a",     "-a",   0, 0, 0);
  bad_name("_a",     "_a",   0, 0, 0);
  bad_name("a-",     "a-",   0, 0, 0);
  bad_name("a_",     "a_",   0, 0, 0);
  bad_name("a__a",   "a__a", 0, 0, 0);
  bad_name("a--a",   "a--a", 0, 0, 0);
  bad_name("a: a--", "a--",  3, 0, 3);
  bad_name("a: 9a",  "9a",   3, 0, 3);
  bad_name("a: 9a",  "9a",   3, 0, 3);
  bad_name("a:\nZ:", "Z",    3, 1, 0);
}
