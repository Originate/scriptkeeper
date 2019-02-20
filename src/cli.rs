use crate::R;
use std::path::PathBuf;

pub enum Args {
    ExecutableMock { executable_mock_path: PathBuf },
    CheckProtocols { script_path: PathBuf },
}

pub fn parse_args(mut args: impl Iterator<Item = String>) -> R<Args> {
    args.next()
        .ok_or("argv: expected program name as argument 0")?;
    Ok(match args.next().ok_or("supply one argument")?.as_ref() {
        "--executable-mock" => Args::ExecutableMock {
            executable_mock_path: PathBuf::from(
                args.next().expect("expected executable file as argument 1"),
            ),
        },
        argument => Args::CheckProtocols {
            script_path: PathBuf::from(argument),
        },
    })
}
