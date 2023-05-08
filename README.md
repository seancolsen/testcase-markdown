# Testcase Markdown

This is a small utility that lets you write test cases for your rust code in markdown files. It might be useful it you're managing a large corpus of test cases for code that operates on strings, such as parsers, formatters, transpilers etc.

## Usage

1. Write some test cases in markdown

    ````md
    # Tests

    ```toml options
    foo = 42
    bar = "This options value gets carried through to all tests in child headings"
    ```

    ## The first test

    ```
    This is the first argument
    ```

    ```
    This is the second argument
    ```

    ## The second test

    ```toml options
    bar = "This is a new options value to be used only within this test"
    ```

    ```
    Argument
    ```

    ```
    blah blah
    ```
    ````

1. In the markdown:

    - Use headings to organize your tests. You can nest them arbitrarily deep.
    - Tag code blocks with `options` to pass them to the options serializer. Options will be inherited by tests under child headings.
    - Pass positional arguments to your test via other code blocks (i.e. _not_ tagged with `options`). These code blocks can have any language associated with them.
    - Headings and code blocks are the only things that matter to the parser. You can use paragraphs to add comments to your tests if you like.

1. Write a test which reads the markdown

    ```rs
    #[cfg(test)]
    mod tests {
        use super::*;
        use testcase_markdown::{get_test_cases, MergeSerialized, TestCase};
        use std::path::PathBuf;
        use toml::{from_str, Table};

        #[derive(Default, PartialEq, Eq, Debug, Clone, Copy)]
        struct Options {
            foo: i64,
            bar: bool,
        }

        impl MergeSerialized for Options {
            fn merge_serialized(&self, source: String) -> Result<Self, String> {
                // Write some logic here to deserialize your options and merge them
                // with higher-level options.
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
            let markdown_content = get_your_markdown_content_from_a_file_or_elsewhere();
            let test_cases = get_test_cases(markdown_content, Options::default());
            for test_case in test_cases {
                // Run your test logic here
            }
        }
    }
    ```

1. Within your test, each test case looks like this:

    ```rs
    pub struct TestCase<Options> {
        pub name: String,
        pub headings: Vec<String>,
        pub line_number: usize,
        pub options: Options,
        pub args: Vec<String>,
    }
    ```
