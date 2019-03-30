use crate::ExitCode;

#[derive(Debug, PartialEq, Clone)]
pub enum CheckerResult {
    Pass,
    Failure(String),
}

impl CheckerResult {
    fn is_pass(&self) -> bool {
        match self {
            CheckerResult::Pass => true,
            CheckerResult::Failure(_) => false,
        }
    }

    fn format(&self, number: Option<usize>) -> String {
        match self {
            CheckerResult::Failure(error) => {
                let header = number.map_or("error".to_string(), |number| {
                    format!("error in test {}", number)
                });
                format!("{}:\n{}", header, error)
            }
            CheckerResult::Pass => match number {
                None => panic!("CheckerResult.format: shouldn't happen"),
                Some(number) => format!("test {}:\n  Tests passed.\n", number),
            },
        }
    }
}

pub struct CheckerResults(pub Vec<CheckerResult>);

impl CheckerResults {
    pub fn format(&self) -> String {
        if self.is_pass() {
            "All tests passed.\n".to_string()
        } else {
            let CheckerResults(results) = &self;
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

    pub fn is_pass(&self) -> bool {
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
