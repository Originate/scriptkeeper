This directory contains all integration tests, bundled in
`tests/integration/main.rs`. The reason they're not directly in `tests` is that
then they would be compiled separately, which takes significantly more time.
