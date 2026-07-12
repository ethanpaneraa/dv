# AGENT.md

Guidance for any coding agent (including future Claude Code sessions) working in this
repo. See [MVP.md](MVP.md) for product scope.

## Project layout

- `src/main.rs` — CLI entry point, terminal setup/teardown, event loop.
- `src/git.rs` — shells out to `git diff` / `git diff --staged`, returns raw text.
- `src/diffmodel.rs` — parses unified diff text into `FileDiff` → `Hunk` → `Line`.
- `src/highlight.rs` — `syntect` wrapper, converts syntax styles into `ratatui` spans.
- `src/app.rs` — app state (`App`), pre-renders each file's diff into styled
  `ratatui::text::Line`s at load time.
- `src/ui.rs` — layout and widget rendering (sidebar + diff pane) for one frame.

Binary is named `dv`, not `diff` — a global install named `diff` would shadow the Unix
`diff` command on `PATH`.

## Before considering any change done

```
cargo build
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

All four must be clean. `cargo fmt` has already been run once over the initial MVP —
keep it that way rather than letting formatting drift and fixing it in a big batch later.

## Rust conventions for this codebase

- **Errors**: `anyhow::Result` at the boundaries (`main`, `git::diff`). Don't introduce
  a custom error enum unless a caller actually needs to match on error variants.
- **Panics**: no `unwrap`/`expect` on data derived from user input, file contents, or
  subprocess output — use `?` or explicit fallback (see `parse_hunk_header`'s
  `unwrap_or(1)` for the pattern: fall back to a sane default rather than crash on a
  malformed hunk header). `unwrap`/`expect` are fine in `#[cfg(test)]` code.
- **Ownership in rendering**: `app.rs` builds `Vec<Line<'static>>` once per file at
  load time rather than re-highlighting on every frame. Keep new rendering work in that
  same "compute once, cache in `App`" shape — don't do per-keystroke recomputation of
  anything that scales with diff size.
- **No new dependencies without checking prior art first.** This project deliberately
  follows the crate choices of `bat`/`delta`/`gitui` (`syntect`, `ratatui`,
  `crossterm`). If a task seems to need a new crate, check whether one of those
  projects already solved it before pulling in something unrelated.
- **Terminal state is sacred.** Any code path that can leave raw mode enabled or the
  alternate screen active after the process exits (including on panic) is a bug. If you
  add new exit paths, route them through the same teardown `main.rs` already does, and
  don't remove the panic hook.

## Testing

- Unit tests live inline in the module they test (`#[cfg(test)] mod tests`), not in a
  separate top-level `tests/` tree, unless a test needs to run the built binary
  end-to-end.
- `diffmodel::parse` is the highest-value thing to unit test — it's pure and has no
  I/O. Add a test case whenever you touch its parsing logic (new file markers, no
  trailing newline, renames, binary files, etc.).
- There's no test coverage yet for `diffmodel::parse` — that's a gap, not a decision;
  add it before extending the parser further.
- For anything touching the TUI itself (`app.rs`, `ui.rs`, `main.rs` event loop), there's
  no automated test harness. Verify manually by running the built binary inside a real
  or `tmux`-allocated pty against a scratch git repo — `ratatui`/`crossterm` need a real
  terminal (`enable_raw_mode` fails under a plain pipe).

## Adding features

Check `MVP.md`'s "Explicitly deferred" list before starting something new — if it's on
that list, confirm scope with the user first rather than assuming it should be built now.
