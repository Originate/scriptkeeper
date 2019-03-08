use crate::ExitCode;

#[derive(Debug, PartialEq)]
pub enum TestResult {
    Pass,
    Failure(String),
}

impl TestResult {
    fn is_pass(&self) -> bool {
        match self {
            TestResult::Pass => true,
            TestResult::Failure(_) => false,
        }
    }

    fn format(&self, number: Option<usize>) -> String {
        match self {
            TestResult::Failure(error) => {
                let header = number.map_or("error".to_string(), |number| {
                    format!("error in protocol {}", number)
                });
                format!("{}:\n{}", header, error)
            }
            TestResult::Pass => match number {
                None => panic!("TestResult.format: shouldn't happen"),
                Some(number) => format!("protocol {}:\n  Tests passed.\n", number),
            },
        }
    }
}

pub struct TestResults(pub Vec<TestResult>);

impl TestResults {
    pub fn format_test_results(&self) -> String {
        if self.is_pass() {
            "All tests passed.\n".to_string()
        } else {
            let TestResults(results) = &self;
            if results.len() == 1 {
                results.iter().next().unwrap().format(None)
            } else {
                results
                    .iter()
                    .enumerate()
                    .map(|(i, result)| result.format(Some(i + 1)))
                    .collect::<Vec<String>>()
                    .join("")
            }
        }
    }

    fn is_pass(&self) -> bool {
        self.0.iter().all(|result| result.is_pass())
    }

    pub fn exitcode(&self) -> ExitCode {
        if self.is_pass() {
            ExitCode(0)
        } else {
            ExitCode(1)
        }
    }
}
