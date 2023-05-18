# Scripts

The script feature makes it possible to post-process bard outputs in arbitrary ways with script files.

Scripts are defined per output. To configure, set the `script` option on the relevant output, for example:

```toml
[[output]]
file = "songbook.html"
script = "foo"
```

The script name is specified **without** the file extension. The extension is assumed automatically based on operating system:

| OS | Script name | Actual filename |
| --- | --- | --- |
| Linux & Unix | `foo` | `foo.sh` |
| Windows | `foo` | `foo.bat` |

The script file should be placed in the output directory and it is also executed in the output directory (ie. with the _current directory_ being set to the output directory).

Note: On Linux/Unix, the script `.sh` file should have the executable permission bit set such that the user running bard can run the script file as well.

### Environment variables

The following environment variables are passed by bard to the script:

| Variable | Purpose |
| --- | --- |
| `OUTPUT` | Full path to the output file for which the script is executed. |
| `OUTPUT_STEM` | Only the 'stem' part of the output filename, ie. the filename without the extension. |
| `OUTPUT_DIR` | Full path to the output directory. |
| `PROJECT_DIR` | Full path to the project directory, ie. where the `bard.toml` file is located. |
| `BARD` | Full path to the bard executable that was called to build the project. |

### Example: All chords in the book

In this example, we'll define a script that will export all the chords in the songbook as a JSON array. We'll be using the `jq` program to do this.

First, let's add a [JSON output](./json-and-xml.md) with a script file set:

```toml
[[output]]
file = "songbook.json"
script = "chords"
```

Then, we'll create a file named `chords.sh`, set the exec bit (`chmod 755 chords.sh`), and write the following contents

```sh
#!/bin/sh

jq '[ .songs[].blocks[].paragraphs[][] | select(.type == "i-chord").chord ] | unique' "$OUTPUT" > "${OUTPUT_STEM}-chords.json"
```

After building the project, a file named `songbook-chords.json` should be generated in the output directory.
It should contain a list similar to this:

```json
[
  "Am",
  "C",
  "C7",
  "D",
  "D7",
  "Dm",
  "Em",
  "F",
  "G",
  "G7"
]
```
