use markdown::{
    mdast::{Heading, Node},
    to_mdast, ParseOptions,
};
use std::mem::take;

pub trait MergeSerialized {
    fn merge_serialized(&self, source: String) -> Result<Self, String>
    where
        Self: Sized;
}

struct Section<Options: MergeSerialized> {
    pub depth: u8,
    pub name: String,
    pub line: usize,
    pub options: Options,
}

struct SectionStack<Options: MergeSerialized + Clone> {
    root_options: Options,
    sections: Vec<Section<Options>>,
}

impl<Options: MergeSerialized + Clone> SectionStack<Options> {
    pub fn new(root_options: Options) -> Self {
        Self {
            root_options,
            sections: Vec::<Section<Options>>::new(),
        }
    }

    pub fn push_heading(&mut self, heading: Heading) {
        let Node::Text(text) = heading.children.into_iter().nth(0).unwrap() else {
            panic!("Markdown headings must contain plain text.")
        };
        let depth = heading.depth;
        self.sections.retain(|s| s.depth < depth);
        let section = Section {
            depth,
            line: heading.position.unwrap().start.line,
            name: text.value,
            options: self.get_options().clone(),
        };
        self.sections.push(section);
    }

    pub fn set_options(&mut self, options: Options) {
        if let Some(last_section) = self.sections.last_mut() {
            last_section.options = options;
        } else {
            self.root_options = options;
        }
    }

    pub fn get_options(&self) -> &Options {
        self.sections
            .last()
            .map(|s| &s.options)
            .unwrap_or_else(|| &self.root_options)
    }

    pub fn get_headings(&self) -> Vec<String> {
        self.sections.iter().map(|s| s.name.clone()).collect()
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct TestCase<Options: MergeSerialized> {
    pub name: String,
    pub headings: Vec<String>,
    pub line_number: usize,
    pub options: Options,
    pub args: Vec<String>,
}

impl<Options: MergeSerialized + Clone> TestCase<Options> {
    fn new(args: Vec<String>, section_stack: &SectionStack<Options>) -> TestCase<Options> {
        let options = section_stack.get_options().clone();
        let mut headings = section_stack.get_headings();
        let name = headings
            .pop()
            .unwrap_or_else(|| "(Unnamed test)".to_string());
        TestCase {
            name,
            headings,
            line_number: section_stack.sections.last().map(|s| s.line).unwrap_or(0),
            options,
            args,
        }
    }
}

pub fn get_test_cases<Options: MergeSerialized + Clone>(
    content: String,
    root_options: Options,
) -> Vec<TestCase<Options>> {
    let ast = to_mdast(&content, &ParseOptions::default()).unwrap();
    let Node::Root(root_node) = ast else {
        panic!("No root node found")
    };
    let nodes = root_node.children;
    let mut section_stack = SectionStack::new(root_options);
    let mut test_cases: Vec<TestCase<Options>> = vec![];
    let mut args: Vec<String> = vec![];
    let mut push_test_case = |s: &SectionStack<Options>, a: &mut Vec<String>| {
        if a.len() > 0 {
            test_cases.push(TestCase::new(take(a), &s));
        }
    };
    for node in nodes {
        match node {
            Node::Heading(heading) => {
                push_test_case(&section_stack, &mut args);
                section_stack.push_heading(heading);
            }
            Node::Code(code) => {
                if code.meta.as_deref() == Some("options") {
                    let options = section_stack
                        .get_options()
                        .merge_serialized(code.value)
                        .unwrap_or_else(|error| {
                            let line = code.position.unwrap().start.line;
                            panic!(
                                "Failed to parse options from code block at line {}: {}",
                                line, error
                            );
                        });
                    section_stack.set_options(options)
                } else {
                    args.push(code.value)
                }
            }
            _ => {}
        }
    }
    push_test_case(&section_stack, &mut args);
    test_cases
}

#[cfg(test)]
mod tests {
    use crate::{get_test_cases, MergeSerialized, TestCase};
    use std::path::PathBuf;
    use toml::{from_str, Table};

    #[derive(Default, PartialEq, Eq, Debug, Clone, Copy)]
    struct Options {
        foo: i64,
        bar: bool,
    }

    impl MergeSerialized for Options {
        fn merge_serialized(&self, source: String) -> Result<Self, String> {
            let values = from_str::<Table>(&source).map_err(|e| e.to_string())?;
            Ok(Options {
                foo: values
                    .get("foo")
                    .and_then(|v| v.as_integer())
                    .unwrap_or(self.foo),
                bar: values
                    .get("bar")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(self.bar),
            })
        }
    }

    #[test]
    fn test_basic() {
        let path = PathBuf::from_iter([env!("CARGO_MANIFEST_DIR"), "src", "test.md"]);
        let content = std::fs::read_to_string(path).unwrap();
        let result = get_test_cases(content, Options::default());
        let expected = [
            TestCase {
                name: "Apple".to_owned(),
                headings: vec!["Tests".to_owned(), "Fruits".to_owned()],
                line_number: 10,
                options: Options { foo: 5, bar: true },
                args: vec!["Granny Smith".to_owned(), "red".to_owned()],
            },
            TestCase {
                name: "Pear".to_owned(),
                headings: vec!["Tests".to_owned(), "Fruits".to_owned()],
                line_number: 20,
                options: Options { foo: 5, bar: false },
                args: vec!["Bartlett".to_owned(), "yellow".to_owned()],
            },
            TestCase {
                name: "Potato".to_owned(),
                headings: vec!["Tests".to_owned(), "Vegetables".to_owned()],
                line_number: 40,
                options: Options { foo: 11, bar: true },
                args: vec!["Russet".to_owned(), "brown".to_owned()],
            },
        ];
        assert_eq!(result, expected);
    }
}
