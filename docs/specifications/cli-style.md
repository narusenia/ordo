# Ordo — CLI Output Style Specification

## Style Modes

Ordo supports three output styles, selectable per-invocation or per-project.

| Mode | Description |
|------|-------------|
| `default` | Modern rich — spinners, Unicode icons, color-coded status. Closer to bun/deno than Cargo. |
| `minimal` | No spinners, no icons on build output. One-line header, Ninja raw output passthrough, dependency resolution displayed as-is. |
| `cargo-like` | Cargo-style right-aligned green verbs, no spinners. `Compiling` lines flow one by one. Dependency fetch uses progress bar. |

### Configuration

**`Ordo.toml`**

```toml
[cli]
style = "default"   # "default" | "minimal" | "cargo-like"
```

**CLI flag (global)**

```
ordo --style minimal build
```

**Environment variable**

```
ORDO_CLI_STYLE=minimal ordo build
```

**Priority**: `--style` > `ORDO_CLI_STYLE` > `Ordo.toml [cli].style` > `default`

When unspecified, the style is `default`.

The style applies to **all commands**.

---

## Mode: `default`

### Tone

Modern rich — spinners, Unicode icons, color-coded status.

### Icons

| State | Icon | Usage |
|-------|------|-------|
| In progress | `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` (braille dots spinner) | Active operations |
| Success | `✔` | Completed actions |
| Failure | `✖` | Failed actions |
| Warning | `⚠` | Skipped, warnings |
| Skip | `⊘` | Nothing to do |
| Run | `▶` | Program execution |

### Layout

Icon-first, bold verb, no colon, no right-align padding:

```
⠹ Compiling src/main.cpp [1/3]
✔ Compiled src/main.cpp [1/3]
✖ Failed src/main.cpp [2/3]
```

### Color Palette

| State | Color | Usage |
|-------|-------|-------|
| Action in progress | **cyan bold** | Compiling, Linking, Fetching, Installing |
| Success | **green bold** | Finished, Created, Compiled, Removed, Running |
| Warning | **yellow bold** | Skipped, Warning |
| Error | **red bold** | Error, Failed |
| Meta / auxiliary | **dim** | Elapsed time, paths, version info, tree lines, `$` commands |

### Spinner Behavior

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

### Build Output

Uses `indicatif::MultiProgress`. Completed files stack upward, in-progress file is at the bottom:

```
✔ Compiled src/main.cpp [1/3]
✔ Compiled src/util.cpp [2/3]
⠹ Compiling src/app.cpp [3/3]
```

Finished line includes profile details and output path:

```
✔ Finished `debug` profile [unoptimized + debuginfo] in 0.53s
  → target/debug/myapp
```

The `→` and path are displayed in dim.

### Error Display

Build errors are wrapped in a `miette::Diagnostic` with the compiler output attached. Ordo emits a structured error (with error code, e.g. `E0200`) and the compiler's raw output is included as context:

```
✔ Compiled src/util.cpp [1/2]
⠹ Compiling src/main.cpp [2/2]

  × Build failed for 'myapp' (E0200)

  src/main.cpp:5:1: error: unknown type name 'intt'
      5 | intt main() {
        | ^~~~

  help: fix the compiler errors above and try again
```

### `ordo new` / `ordo init`

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

---

## Mode: `minimal`

No spinners, no icons on build steps. Dependency resolution is displayed as-is (same as `default`). Ninja raw output is passed through directly.

### Build Output

**Success:**

```
Building 'myapp'...
  ✔ Resolved  fmt 11.1.1 (vcpkg)
  ✔ Resolved  spdlog 1.15.3 (vcpkg)
  ✔ Resolved  raylib (git: github.com/raysan5/raylib@5.5)
[1/5] Compiling src/main.cpp
[2/5] Compiling src/utils.cpp
[3/5] Compiling src/app.cpp
[4/5] Compiling src/render.cpp
[5/5] Linking myapp
Finished dev [debug] in 2.34s
```

**Failure:**

```
Building 'myapp'...
  ✔ Resolved  fmt 11.1.1 (vcpkg)
[1/5] Compiling src/main.cpp
[2/5] Compiling src/app.cpp
src/app.cpp:12:5: error: use of undeclared identifier 'foo'
    foo();
    ^
1 error generated.
Build failed.
```

Errors are compiler raw output — no `Diagnostic` wrapping, no decoration.

### Other Commands

Commands with little output (`clean`, `new`, `add`, etc.) use the same text content as `default` but without spinners. Progress bars are hidden (`ProgressBar::hidden()`).

---

## Mode: `cargo-like`

Mimics Cargo's output style: right-aligned 12-character-wide green verbs, no spinners, no icons. Dependency fetch uses a progress bar.

### Build Output

**Success:**

```
  Downloading 3 packages...
  Downloaded fmt v11.1.1 [======>  ] 2/3
  Downloaded spdlog v1.15.3 [========>] 3/3
   Compiling src/main.cpp
   Compiling src/utils.cpp
   Compiling src/app.cpp
   Compiling src/render.cpp
     Linking myapp
    Finished dev [debug] target(s) in 2.34s
```

**Failure:**

```
   Compiling src/main.cpp
   Compiling src/app.cpp
error: could not compile 'myapp'

src/app.cpp:12:5: error: use of undeclared identifier 'foo'
    foo();
    ^
1 error generated.
```

### Other Commands

```
     Created C++ executable project `myapp`
     Removed target/ (128 MB freed)
     Running target/debug/myapp
```

Verbs are right-aligned to 12 characters and colored green.

---

## Verbosity Levels

Verbosity is orthogonal to style mode. All modes support `-v` / `-vv`.

| Level | Output |
|-------|--------|
| Default | Style-specific output only |
| `-v` | Above + executed commands in dim with `$` prefix |
| `-vv` | Above + tracing debug logs |

Example with `-v` (shown in `default` mode):

```
✔ Compiled src/main.cpp [1/1]
  $ clang++ -c -std=c++20 -O0 -g -o main.o ../../../src/main.cpp
✔ Linked myapp
  $ clang++ -o ../myapp main.o
```

---

## Implementation

### Architecture

`trait StyleOutput` with three implementations: `DefaultStyle`, `MinimalStyle`, `CargoLikeStyle`.

```rust
trait StyleOutput {
    fn success(&self, verb: &str, msg: &str);
    fn error(&self, verb: &str, msg: &str);
    fn warn(&self, verb: &str, msg: &str);
    fn skip(&self, verb: &str, msg: &str);
    fn header(&self, msg: &str);
    fn meta(&self, msg: &str);
    fn create_spinner(&self, msg: &str) -> ProgressBar;
    fn create_progress_bar(&self, total: u64, msg: &str) -> ProgressBar;
    fn finish_spinner_success(&self, pb: &ProgressBar, verb: &str, msg: &str);
    fn finish_spinner_error(&self, pb: &ProgressBar, verb: &str, msg: &str);
    fn display_build_step(&self, action: &str, file: &str, current: u32, total: u32);
    // ...
}
```

- `MinimalStyle` returns `ProgressBar::hidden()` for spinners (no-op).
- `CargoLikeStyle` returns `ProgressBar::hidden()` for spinners, real `ProgressBar` for dependency fetch progress.
- Ninja output parsing is shared across all modes. Parsed result (action, file, progress) is passed to `display_build_step()`.

### Context

```rust
struct Context {
    style: Box<dyn StyleOutput>,
    verbose: u8,
    color: ColorMode,
}
```

`Context` is constructed in `main.rs` and passed to all command `run()` functions.

### Crates

- `console` — styled text and terminal control
- `indicatif` — spinners (`ProgressBar`) and multi-progress (`MultiProgress`)
- `miette` — `Diagnostic` error display (`default` mode build errors)
- `src/util/style.rs` — central style module, trait definition and implementations
