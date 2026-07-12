# AGENT.md

Guidance for any coding agent (including future Claude Code sessions) working in this
repo. See [MVP.md](MVP.md) for product scope.

## Project layout

- `src/main.rs` — entry point, terminal setup/teardown, event loop. Two paths: an
  explicit `dv <path>` argument skips discovery and Home entirely (for scripting/pager
  use); no path means auto-discovery (cwd + its immediate git-repo children with
  changes) followed by `Screen::Home`.
- `src/git.rs` — shells out to `git diff` / `git diff --staged` in a given directory
  (`diff_in`), plus `is_repo` for repo detection.
- `src/diffmodel.rs` — parses unified diff text into `FileDiff` → `Hunk` → `Line`.
- `src/project.rs` — project discovery (`discover`: immediate git-repo subdirectories of
  a scan dir) and loading (`load`: runs the diff for one root, `None` if not a repo or
  no changes).
- `src/highlight.rs` — `syntect` wrapper, converts syntax styles into `ratatui` spans.
- `src/app.rs` — app state (`App` holding `Vec<ProjectView>` plus a `Screen` enum:
  `Home` or `Diff`). Pre-renders each file's diff into styled `ratatui::text::Line`s at
  load time. Home-screen nav state (`query`/`matches`/`matched_selected`) and its
  type/backspace/move/confirm methods; `go_home()` returns to it from `Diff`.
- `src/ui.rs` — `draw()` dispatches on `app.screen`: `draw_home()` is a full-page
  dashboard (ASCII logo, filter input, project list, footer) with no floating widgets;
  `draw_diff_screen()` renders Files + Diff panes plus a footer hint bar.

This went through two rejected iterations before landing on the current design — don't
reintroduce either:
1. A permanent Projects sidebar column: ate width from the diff pane, split navigation
   across two key models ({}/} vs n/p).
2. A floating command-palette overlay (`Clear` + centered `Rect` on top of the diff):
   fixed the width problem but was still a modal bolted onto content, not an actual app
   screen — the user explicitly wants this "more like a TUI, less like a CLI," closer to
   nvim's dashboard than to a fuzzy-finder popup.

The current model: `Screen::Home` is a real full-page screen (like nvim's dashboard,
shown even when there's only one project, for consistency), and `Screen::Diff` is the
Files+Diff view. Switching between them is a screen replacement, not an overlay. Extend
`Screen` with new variants for new full-page views rather than drawing more floating
boxes.

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

- **Errors**: `anyhow::Result` at the boundaries (`main`, `git::diff_in`). Don't introduce
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
