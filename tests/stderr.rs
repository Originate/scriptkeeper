#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![deny(clippy::all)]

#[path = "./utils.rs"]
mod utils;

use check_protocols::R;
use test_utils::trim_margin;
use utils::test_run_c;

mod file_descriptor_2 {
    use super::*;

    #[test]
    fn captures_writes_to_file_descriptor_2() -> R<()> {
        test_run_c(
            r##"
                |#include <stdio.h>
                |int main() {
                |  fprintf(stderr, "bar\n");
                |  return 0;
                |}
            "##,
            r##"
                |- protocol: []
                |  stderr: "foo\n"
            "##,
            Err(&trim_margin(
                "
                    |error:
                    |  expected output to stderr:
                    |foo
                    |  received output to stderr:
                    |bar
                ",
            )?),
        )?;
        Ok(())
    }

    #[test]
    fn passes_for_expected_writes() -> R<()> {
        test_run_c(
            r##"
                |#include <stdio.h>
                |int main() {
                |  fprintf(stderr, "foo\n");
                |  return 0;
                |}
            "##,
            r##"
                |- protocol: []
                |  stderr: "foo\n"
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn does_not_capture_writes_to_stdout() -> R<()> {
        test_run_c(
            r##"
                |#include <stdio.h>
                |int main() {
                |  printf("foo\n");
                |  return 0;
                |}
            "##,
            r##"
                |- protocol: []
                |  stderr: "foo\n"
            "##,
            Err(&trim_margin(
                "
                    |error:
                    |  expected output to stderr:
                    |foo
                    |  received output to stderr:
                ",
            )?),
        )?;
        Ok(())
    }
}
