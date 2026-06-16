# Ordo — CLI Output Style Specification

## Tone

Modern rich — spinners, Unicode icons, color-coded status. Closer to bun/deno than Cargo.

## Icons

| State | Icon | Usage |
|-------|------|-------|
| In progress | `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` (braille dots spinner) | Active operations |
| Success | `✔` | Completed actions |
| Failure | `✖` | Failed actions |
| Warning | `⚠` | Skipped, warnings |
| Skip | `⊘` | Nothing to do |
| Run | `▶` | Program execution |

## Layout

Icon-first, bold verb, no colon, no right-align padding:

```
⠹ Compiling src/main.cpp [1/3]
✔ Compiled src/main.cpp [1/3]
✖ Failed src/main.cpp [2/3]
```

## Color Palette

| State | Color | Usage |
|-------|-------|-------|
| Action in progress | **cyan bold** | Compiling, Linking, Fetching, Installing |
| Success | **green bold** | Finished, Created, Compiled, Removed, Running |
| Warning | **yellow bold** | Skipped, Warning |
| Error | **red bold** | Error, Failed |
| Meta / auxiliary | **dim** | Elapsed time, paths, version info, tree lines, `$` commands |

## Spinner Behavior

Spinners are used for operations that take noticeable time:

| Command | Spinner | Notes |
|---------|---------|-------|
| `ordo build` | Yes | Per-file progress via ninja stdout parsing |
| `ordo run` | Build phase only | `▶ Running` is static (program output takes over) |
| `ordo test` | Yes | Per-test-binary progress |
| `ordo fmt` | Yes | If many files |
| `ordo lint` | Yes | If many files |
| `ordo clean` | No | Instant |
| `ordo new` | No | Instant |
| Dependency fetch | Yes + progress bar | Future |

On completion, the spinner line is replaced:

- **Success**: icon changes to `✔`, verb changes to past tense, color changes to green
  - `⠹ Compiling src/main.cpp [1/3]` → `✔ Compiled src/main.cpp [1/3]`
- **Failure**: icon changes to `✖`, color changes to red
  - `⠹ Compiling src/main.cpp [2/3]` → `✖ Failed src/main.cpp [2/3]`

## Build Output

### File-level Progress

Uses `indicatif::MultiProgress`. Completed files stack upward, in-progress file is at the bottom:

```
✔ Compiled src/main.cpp [1/3]
✔ Compiled src/util.cpp [2/3]
⠹ Compiling src/app.cpp [3/3]
```

### Ninja Integration

- Ninja stdout is piped and parsed for `[N/M]` progress patterns
- Ninja stderr is passed through for compiler error output
- Ordo renders its own progress UI based on parsed status

### Finished Line

Includes profile details and output path:

```
✔ Finished `debug` profile [unoptimized + debuginfo] in 0.53s
  → target/debug/myapp
```

The `→` and path are displayed in dim.

### Error Display

Errors are shown inline in the progress flow. Compiler output is unmodified:

```
✔ Compiled src/util.cpp [1/2]
✖ Failed src/main.cpp [2/2]
  src/main.cpp:5:1: error: unknown type name 'intt'
      5 | intt main() {
        | ^~~~
✖ Build failed
```

## Command-Specific Output

### `ordo new` / `ordo init`

Tree display of generated files (tree lines in dim):

```
✔ Created C++ executable project `myapp`
  myapp/
  ├── Ordo.toml
  ├── src/
  │   └── main.cpp
  ├── tests/
  │   └── main_test.cpp
  └── .gitignore
```

### `ordo run`

```
✔ Finished `debug` profile in 0.05s
  → target/debug/myapp
▶ Running target/debug/myapp
Hello, world!
```

Program output is never modified by Ordo.

### `ordo test`

Per-file results with summary bar:

```
✔ Passed main_test [1/3]
✔ Passed util_test [2/3]
✖ Failed app_test [3/3]
  assertion failed: expected 1, got 2

━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Tests: 2 passed, 1 failed
  Time:  0.12s
```

Summary bar (`━━━`) is dim.

### `ordo clean`

```
✔ Removed target/ (128 MB freed)
```

When nothing to clean:

```
⊘ Nothing to clean
```

## Verbosity Levels

| Level | Output |
|-------|--------|
| Default | Icons + verbs + summary only |
| `-v` | Above + executed commands in dim with `$` prefix |
| `-vv` | Above + tracing debug logs |

Example with `-v`:

```
✔ Compiled src/main.cpp [1/1]
  $ clang++ -c -std=c++20 -O0 -g -o main.o ../../../src/main.cpp
✔ Linked myapp
  $ clang++ -o ../myapp main.o
```

## Implementation

- `console` crate for styled text and terminal control
- `indicatif` crate for spinners (`ProgressBar`) and multi-progress (`MultiProgress`)
- `src/util/style.rs` as the central style module
