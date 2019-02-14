# `check-protocols`

## Status

**This tool is very experimental. It may give incorrect results and delete your
files.**

There's lots and lots of features still missing from `check-protocols`. If you
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

Given that, you can create a protocol file `./foo.sh.protocol.yaml`:

```yaml
- /bin/ls
- /usr/bin/docker run --rm -it hello-world
```

Now running `check-protocols ./foo.sh` will tell you whether your script
`./foo.sh` conforms to your protocol `./foo.sh.protocol.yaml`.

There's more example test cases in the [tests](./tests) folder.
