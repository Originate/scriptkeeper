# `scriptkeeper`
[![CircleCI](https://circleci.com/gh/Originate/scriptkeeper.svg?style=svg)](https://circleci.com/gh/Originate/scriptkeeper)
[![open issues](https://img.shields.io/github/issues/originate/scriptkeeper.svg?label=open%20issues)](https://github.com/Originate/scriptkeeper/issues)
[![open pull requests](https://img.shields.io/github/issues-pr-raw/originate/scriptkeeper.svg?color=%238888ff)](https://github.com/Originate/scriptkeeper/pulls)
[![closed issues](https://img.shields.io/github/issues-closed/originate/scriptkeeper.svg?color=green&label=closed%20issues)](https://github.com/Originate/scriptkeeper/issues?q=is%3Aissue+is%3Aclosed)
[![issue board](https://img.shields.io/badge/scriptkeeper-issue%20board-important.svg)](https://github.com/orgs/Originate/projects/1)

Run tests against your scripts without changing your scripts.

## Description

Automated tests help us write well-behaved applications, and that's great, but
what about all those pesky little scripts we use in and around our applications
(e.g. deploy scripts)? How do we test those?

`scriptkeeper` is a tool for people who wish to write tests for existing
scripts and/or use TDD to write new scripts. Because of its design,
`scriptkeeper` is language agnostic, since it mocks out syscalls. That means
you can test Bash scripts just as well as Python, Ruby, etc.


## Status

**This tool is very experimental. It may give incorrect results and delete your
files.**

There are lots and lots of features still missing from `scriptkeeper`. If you
try it out, I'd be interested to hear which features you would want the most.
Feel free to open (or vote on)
[issues](https://github.com/Originate/scriptkeeper/issues).

## Usage

`scriptkeeper` allows you to write tests for scripts (or other executables) --
without the need to modify your executables.

Here's an example script `./build-image.sh`:

```shell
#!/usr/bin/env bash

if [ -z "$(git status --porcelain)" ] ; then
  commit=$(git rev-parse HEAD)
  docker build --tag image_name:$commit .
else
  exit 1
fi
```

For `scriptkeeper` to be able to test this script, you need to add a yaml file
in the same directory as the script that has the additional file extension
`.test.yaml`. So here's the  matching test file
`./build-image.sh.test.yaml`:

```yaml
tests:
  # builds a docker image when git repo is clean
  - steps:
    - command: /usr/bin/git status --porcelain
      stdout: ""
    - command: /usr/bin/git rev-parse HEAD
      stdout: "mock_commit_hash\n"
    - /usr/bin/docker build --tag image_name:mock_commit_hash .
  # aborts when git repo is not clean
  - steps:
    - command: /usr/bin/git status --porcelain
      stdout: " M some-file"
    exitcode: 1
```

Now running `scriptkeeper ./build-image.sh` will tell you whether your script
`./build-image.sh` conforms to your tests in
`./build-image.sh.test.yaml`.

There are more example test cases in the [tests/examples](./tests/examples)
folder.

### `.test.yaml` format

Here's all the fields that are available in the yaml declarations for the
tests: (`?` marks optional fields.)

``` yaml
tests:
  - arguments?: string
      # List of arguments given to the tested script, seperated by spaces.
      # Example: "-rf /", default: ""
    env?:
      # Environment being passed into the tested script.
      # Example: PREFIX: /usr/local/, default: {}
      { [string]: string }
    cwd?: string
      # Current working directory the tested script will be executed in.
      # Example: /test-dir, default: same directory that `scriptkeeper` is run in.
    mockedFiles?: [string]
      # List of files and folders that are going to be mocked to exist.
      # Note that directories must include a trailing '/'.
      # Example: ["/www/logs"], default: []
    stderr?: string
      # Output that the script is expected to write to stderr.
      # Example: "error message\n", default: stderr output is not checked.
    exitcode?: number
      # Exitcode that the tested script is expected to exit with.
      # Default: 0.
    steps:
      # List of commands that your script is expected to execute.
      - command|regex: string
          # One of either `command` or `regex` is required
          #
          # command: the executable, followed by its arguments, separated by spaces.
          # Example: /bin/chmod +x foo.sh
          #
          # regex: a regular expression (for valid syntax, see: https://docs.rs/regex/1.1.2/regex/#syntax)
          # Note that the regex is automatically anchored, so it must match the entire command and its arguments
          # Example: /bin/echo \d+
        stdout?: string
          # Mocked output of this command.
          # Default: ""
        exitcode?: number
          # Mocked exitcode of the command.
          # Default: 0
interpreter?: string
    # The interpreter that should be used to run the tested script.
    # Example: "/bin/bash", default: The program itself will be executed
    # directly, without an interpreter. In that case it has to have the
    # executable flag set. Often you also will need a hashbang.
unmockedCommands: [string]
  # List of executables that are not going to be mocked out, but are going to be
  # executed instead.
  # Example: ["sed", "awk"], default: [].
```

#### Shorthands

For convenience you can specify commands as a string directly. So this

``` yaml
steps:
  - command: git add .
  - command: git push
```

can be written as

``` yaml
steps:
  - git add .
  - git push
```

#### Multiple tests

Multiple tests can be specified using a YAML array:

``` yaml
# when given the 'push' argument, it pushes to the remote
- arguments: push
  steps:
    - git add .
    - git push
# when given the 'pull' argument, it just pulls
- arguments: push
  steps:
    - git pull
```

You can also put everything into a `tests` field:

``` yaml
tests:
  # when given the 'push' argument, it pushes to the remote
  - arguments: push
    steps:
      - git add .
      - git push
  # when given the 'pull' argument, it just pulls
  - arguments: push
    steps:
      - git pull
```

## Recording tests

There is **experimental** support for recording tests. You can either record
tests by passing in the `--record` command line flag, or you can put so-called
holes into your tests:

``` yaml
tests:
  - steps:
      - _
```

This will actually execute the sub-commands that your script performs, without
mocking them out. And it will overwrite your tests file with the recorded
version.

You can also start with a partial test and have `scriptkeeper` fill in the
specified holes:

``` yaml
tests:
  - arguments: foo
    steps:
      - git add .
      - _
```

This allows for an iterative process to create a test:

1. Start with an empty test with a hole.
2. Run `scriptkeeper`.
3. Identify the step in the recorded test where it deviates from the
   intended test. (If it doesn't, you're done.)
4. Refine the test by modifying the inputs to the tested script, i.e. the
   arguments, the environment, etc. This can be guided by both the recorded
   script and the script's output to `stdout` and `stderr`.
5. Remove all test steps after the step identified in 3.
6. Add a hole at the end.
7. Re-iterate from step 2.

## Running inside `docker` (for OSX)

You can run the tool inside docker, for example like this:

``` bash
./build-docker-image.sh
./scriptkeeper-in-docker.sh <PATH_TO_YOUR_SCRIPT>
```

## Contributing

Contributions, feature requests, bug reports, etc. are all welcome.  Please
consider the following guidelines when submitting:

* For any pull request that you intend to merge, we ask that all tests pass for every commit that will end up on master.
* We will address the top rated (by :thumbsup:) issues first, please cast your votes!
* You can coordinate your work through this [ticket board](https://github.com/orgs/Originate/projects/1).

### Running the test suite

You can use [just](https://github.com/casey/just) to run the tests during
development with:

`just dev`

or -- if you want to specify a pattern for which tests to run:

`just dev PATTERN`

To run all checks that would be run on CI, you can do:

`just ci`

### For OSX

This tool does not currently compile or run on OSX. In order to develop on a Mac you will need to
run inside of docker. Luckily, we have set up a one-liner for you. This will run the tests continuously,
within docker, when files change:

``` bash
just build_docker_image
./test-watch-in-docker.sh
```

### Cutting a new release

To cut a new release:

- Bump the version number in the `Cargo.toml`
- Run `just ci`
- `git tag` with the version number (no leading `v`)
- Run `git push --tags`
- Create a release on github for the created tag
- Run `just distribution_build`
- Upload `distribution/scriptkeeper` as a binary release to github

There's a script `distribution/smoke-test.sh` that allows to smoke-test the
distribution executable in a bunch of different docker images. This can be used
to make sure that all dynamically linked dependencies are available on those
systems.
