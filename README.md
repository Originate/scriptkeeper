# Check Protocols
[![CircleCI](https://circleci.com/gh/Originate/check-protocols.svg?style=svg)](https://circleci.com/gh/Originate/check-protocols)
[![open issues](https://img.shields.io/github/issues/originate/check-protocols.svg?label=open%20issues)](https://github.com/Originate/check-protocols/issues)
[![open pull requests](https://img.shields.io/github/issues-pr-raw/originate/check-protocols.svg?color=%238888ff)](https://github.com/Originate/check-protocols/pulls)
[![closed issues](https://img.shields.io/github/issues-closed/originate/check-protocols.svg?color=green&label=closed%20issues)](https://github.com/Originate/check-protocols/issues?q=is%3Aissue+is%3Aclosed)
[![issue board](https://img.shields.io/badge/check--protocols-issue%20board-important.svg)](https://github.com/orgs/Originate/projects/1)

Run tests against your scripts without changing your scripts.

## Description

Automated tests help us write well-behaved applications, and that's great, but
what about all those pesky little scripts we use in and around our applications
(e.g. deploy scripts)? How do we test those?

`check-protocols` is a tool for people who wish to write tests for existing
scripts and/or use TDD to write new scripts. Because of its design,
`check-protocols` is language agnostic, since it mocks out syscalls. That means
you can test Bash scripts just as well as Python, Ruby, etc.


## Status

**This tool is very experimental. It may give incorrect results and delete your
files.**

There are lots and lots of features still missing from `check-protocols`. If you
try it out, I'd be interested to hear which features you would want the most.
Feel free to open (or vote on)
[issues](https://github.com/Originate/check-protocols/issues).

## Usage

`check-protocols` allows you to write tests -- so-called protocols -- for
scripts (or other executables) -- without the need to modify your executables.

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

And here's a matching protocols file `./build-image.sh.protocols.yaml`:

```yaml
protocols:
  # builds a docker image when git repo is clean
  - protocol:
    - command: /usr/bin/git status --porcelain
      stdout: ""
    - command: /usr/bin/git rev-parse HEAD
      stdout: "mock_commit_hash\n"
    - /usr/bin/docker build --tag image_name:mock_commit_hash .
  # aborts when git repo is not clean
  - protocol:
    - command: /usr/bin/git status --porcelain
      stdout: " M some-file"
    exitcode: 1
```

Now running `check-protocols ./build-image.sh` will tell you whether your script
`./build-image.sh` conforms to your protocols in
`./build-image.sh.protocols.yaml`.

There are more example test cases in the [tests/examples](./tests/examples)
folder.

### `.protocols.yaml` format

Here's all the fields that are available in the yaml declarations for the
protocols: (`?` marks optional fields.)

``` yaml
protocols:
  - arguments?: string
      # List of arguments given to the tested script, seperated by spaces.
      # Example: "-rf /", default: ""
    env?:
      # Environment being passed into the tested script.
      # Example: PREFIX: /usr/local/, default: {}
      { [string]: string }
    cwd?: string
      # Current working directory the tested script will be executed in.
      # Example: /test-dir, default: same directory that `check-protocols` is run in.
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
    protocol:
      # List of commands that your script is expected to execute.
      - command: string
          # the executable, followed by its arguments, separated by spaces.
          # Example: /bin/chmod +x foo.sh
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
protocol:
  - command: git add .
  - command: git push
```

can be written as

``` yaml
protocol:
  - git add .
  - git push
```

#### Multiple protocols

Multiple protocols can be specified using a YAML array:

``` yaml
# when given the 'push' argument, it pushes to the remote
- arguments: push
  protocol:
    - git add .
    - git push
# when given the 'pull' argument, it just pulls
- arguments: push
  protocol:
    - git pull
```

You can also put everything into a `protocols` field:

``` yaml
protocols:
  # when given the 'push' argument, it pushes to the remote
  - arguments: push
    protocol:
      - git add .
      - git push
  # when given the 'pull' argument, it just pulls
  - arguments: push
    protocol:
      - git pull
```

## Recording protocols

There is **experimental** support for recording protocols. You can either record
protocols by passing in the `--record` command line flag, or you can put
so-called holes into your protocols:

``` yaml
protocols:
  - protocol:
      - _
```

This will actually execute the sub-commands that your script performs, without
mocking them out. And it will overwrite your protocols file with the recorded
version.

You can also start with a partial protocol and have `check-protocols` fill in
the specified holes:

``` yaml
protocols:
  - arguments: foo
    protocol:
      - git add .
      - _
```

This allows for an iterative process to create a protocol:

1. Start with an empty protocol with a hole.
2. Run `check-protocols`.
3. Identify the step in the recorded protocol where it deviates from the
   intended test. (If it doesn't, you're done.)
4. Refine the protocol by modifying the inputs to the tested script, i.e. the
   arguments, the environment, etc. This can be guided by both the recorded
   script and the script's output to `stdout` and `stderr`.
5. Remove all protocol steps after the step identified in 3.
6. Add a hole at the end.
7. Re-iterate from step 2.

## Running inside `docker` (for OSX)

You can run the tool inside docker, for example like this:

``` bash
./build-docker-image.sh
./check-protocols-in-docker.sh <PATH_TO_YOUR_SCRIPT>
```

## Contributing



Contributions, feature requests, bug reports, etc. are all welcome.  Please
consider the following guidelines when submitting:

* For any pull request that you intend to merge, we ask that all tests pass for every commit that will end up on master.
* We will address the top rated (by :thumbsup:) issues first, please cast your votes!
* You can coordinate your work through this [ticket board](https://github.com/orgs/Originate/projects/1).

### For OSX

This tool does not currently compile or run on OSX. In order to develop on a Mac you will need to
run inside of docker. Luckily, we have set up a one-liner for you. This will run the tests continuously,
within docker, when files change:

``` bash
./build-docker-image.sh
./test-watch-in-docker.sh
```
