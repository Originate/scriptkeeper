# `check-protocols`

 [![Waffle.io - Columns and their card count](https://badge.waffle.io/Originate/check-protocols.svg?columns=all)](https://waffle.io/Originate/check-protocols)

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

Here's an example script `./foo.sh`:

```shell
#!/usr/bin/env bash

ls
docker run --rm -it hello-world
```

Given that, you can create a protocols file `./foo.sh.protocols.yaml`:

```yaml
protocol:
  - /bin/ls
  - /usr/bin/docker run --rm -it hello-world
```

Now running `check-protocols ./foo.sh` will tell you whether your script
`./foo.sh` conforms to your protocols in `./foo.sh.protocols.yaml`.

There are more example test cases in the [tests/examples](./tests/examples)
folder.

### `.protocols.yaml` format

Here's all the fields that are available in the yaml declarations for a
protocol: (`?` marks optional fields.)

``` yaml
arguments?: string
  # List of arguments given to the tested script, seperated by spaces.
  # Example: "-rf /", default: ""
env?:
  # Environment being passed into the tested script.
  # Example: PREFIX: /usr/local/, default: {}
  { [string]: string }
cwd?: string
  # Current working directory the tested script will be executed in.
  # Example: /test-dir, default: same directory that `check-protocols` is run in.
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

## Running inside `docker` (for OSX)

You can run the tool inside docker, for example like this:

``` bash
./build-docker-image.sh
./check-protocols-in-docker.sh <PATH_TO_YOUR_SCRIPT>
```

## Contributing

Contributions, feature requests, bug reports, etc. are all welcome. Please consider the following guidelines
when submitting:

* For any pull request that you intend to merge, we ask that all tests pass for every commit that will end up on master
* We will address the top rated (by :thumbsup:) issues first, please cast your votes!

### For OSX

This tool does not currently compile or run on OSX. In order to develop on a Mac you will need to
run inside of docker. Luckily, we have set up a one-liner for you. This will run the tests continuously,
within docker, when files change:

``` bash
./build-docker-image.sh
./test-watch-in-docker.sh
```
