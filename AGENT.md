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
  no changes). `main.rs` calls `load` for each discovered root in parallel via
  `std::thread::scope` — each call is an independent `git diff` subprocess spawn, and
  running them sequentially was measured to dominate startup time on multi-repo scans.
- `src/highlight.rs` — `syntect` wrapper, converts syntax styles into `ratatui` spans.
- `src/watch.rs` — `spawn(roots, staged)` starts a detached `std::thread` that re-diffs
  every root every 2s and sends `Update { root, project }` over an `mpsc::Receiver`.
  Errors from a single tick's `project::load` (e.g. git transiently locked) are
  swallowed for that root and retried next interval, not surfaced or retried faster.
- `src/app.rs` — app state (`App` holding `Vec<ProjectView>` plus a `Screen` enum:
  `Home` or `Diff`). Rendering (syntax highlighting) is **lazy**: `ProjectView.rendered`
  is `Option<Vec<Vec<Line>>>`, populated by `ensure_rendered()` the first time a project
  is actually opened (`home_confirm`, the only way to switch projects now — see the
  `{`/`}` note below), not at load time — Home never needs it. `App.highlighter:
  Option<Highlighter>` is lazy for the same
  reason: constructing it loads `syntect`'s default syntax/theme sets, which isn't free.
  Home-screen nav state (`query`/`matches`/`matched_selected`) and its type/backspace/
  move/confirm methods; `go_home()` returns to it from `Diff`. `apply_watch_update()`
  merges a fresh diff from `watch.rs` into the matching `ProjectView` (matched by
  `root: PathBuf`, brought back specifically for this after being removed earlier as
  unused — re-add data fields when a real need shows up, don't keep them "for later").
  It's a no-op if the diff didn't actually change (`FileDiff` etc. derive `PartialEq`
  for this) and never removes a project whose changes got committed away — see the
  watch-mode note below for why.
- `src/ui.rs` — `draw()` takes `&mut App` (not `&App`) and dispatches on `app.screen`:
  the Files and Projects lists need mutable access to their persisted `ListState` (see
  the `ListState` note below). `draw_home()` is a full-page dashboard: logo, a stat
  line, a Projects list (filter input + per-project `+N -M`, accent-colored border)
  beside a live Preview pane (selected project's files with their own `+N -M`), and a
  footer with accent-colored key hints. `draw_diff_screen()` renders Files (dim
  border) + Diff (accent border — see below) plus a footer hint bar. One `ACCENT`
  color constant ties selection highlight, the Diff pane's border, the Projects list
  border, and footer key labels together — don't introduce new ad hoc colors for these
  roles, reuse `ACCENT`/`DIM`/`ADDED_FG`/`REMOVED_FG`.

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

**Visual hierarchy, not a focus-toggle model.** The Diff pane's border is accent-colored
and Files' is dim on purpose — Diff is always the primary, always-live content (`j`/`k`
always scroll it, unconditionally), Files is a secondary nav rail (`n`/`p`/arrows always
switch files, unconditionally). There's deliberately no `Focus` enum or Tab-to-switch —
the two key sets were never actually ambiguous, so a toggle would add a mode without
fixing a real conflict. Don't add one speculatively; if a genuine conflict shows up
(e.g. Files needs independent arrow-key navigation), that's when a real `Focus` enum
earns its complexity.

**There is no bare-key project switch from the Diff view, and don't re-add one.**
`{`/`}` used to cycle projects directly without going through Home. It caused two
separate real confusions: first, "these keys don't do anything" (correctly a no-op
with one project loaded, but indistinguishable from broken), then worse, "my files
disappeared" (with multiple projects loaded, it silently swapped the entire Files
list to a different project's with no visual cue a switch happened). Removed
entirely rather than patched with a visual indicator — two confusing incidents from
one feature was enough signal it wasn't earning its complexity. The only way to
switch projects now is `go_home()` (double-tap Space) → pick explicitly on the full
Home screen, where you can see what you're choosing before committing. If a quick
in-place switcher gets re-requested, that's a product decision to raise with the
user, not something to quietly rebuild.

**Watch mode is polling, not filesystem events, on purpose.** `notify` (or similar)
would react instantly instead of within a 2s window, but `std::thread` + a plain
re-diff loop needed zero new dependencies and was simple enough to get right the first
time. If the interval ever feels too slow, that's the point to evaluate `notify` — don't
add it speculatively. Two behavior constraints exist specifically to keep a live-updating
list from being disorienting mid-review, and both are intentional, not oversights:
watch updates never remove a project from the list (a project whose changes got
committed away keeps showing its last content), and the watcher never discovers new
repos that appear under the scan root after startup — it only re-diffs the roots found
at launch.

Binary is named `dv`, not `diff` — a global install named `diff` would shadow the Unix
`diff` command on `PATH`. Toolchain is pinned via `rust-toolchain.toml` (1.97.0) — this
project needed 1.85+ for transitive deps (`edition2024`), so a bare `cargo build` on an
older system-wide toolchain would otherwise fail exactly like it did at project start.

**Any `ratatui` `List`/`Table` the user scrolls through needs a `ListState` persisted
in `App`, not a fresh `ListState::default()` built inside the render function.** We
shipped the Files sidebar and Projects list with a fresh one every frame; it looked
broken — the moment the selection scrolled past the visible window, ratatui recomputed
the viewport from a blank slate instead of nudging it by one line, so everything above
the selection appeared to vanish in one jump instead of scrolling. Fixed by adding
`files_list_state`/`project_list_state: ListState` fields to `App` and reusing them
across frames (`ui::draw` takes `&mut App` for exactly this reason now). If you add
another scrollable list, give it the same treatment up front.

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
- **Ownership in rendering**: `app.rs` builds `Vec<Line<'static>>` once per file, lazily
  on first visit (`ensure_rendered`), and caches it rather than re-highlighting on every
  frame. Keep new rendering work in that same "compute once on first need, cache in
  `App`" shape — don't do per-keystroke recomputation of anything that scales with diff
  size, and don't move work back to eager/upfront unless it's genuinely needed before
  the first screen draws (Home doesn't need any file content).
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
  I/O. It now has coverage for new/deleted/renamed/binary files and "no newline at end
  of file," built from real `git diff` output captured against scratch repos rather
  than hand-written — extend the same way (generate real fixtures, don't guess git's
  format) when you touch parsing logic further, e.g. for a future rename-metadata
  display or Jujutsu/Sapling support with a different diff format.
- For anything touching the TUI itself (`app.rs`, `ui.rs`, `main.rs` event loop), there's
  no automated test harness. Verify manually by running the built binary inside a real
  or `tmux`-allocated pty against a scratch git repo — `ratatui`/`crossterm` need a real
  terminal (`enable_raw_mode` fails under a plain pipe).

## Adding features

Check `MVP.md`'s "Explicitly deferred" list before starting something new — if it's on
that list, confirm scope with the user first rather than assuming it should be built now.
