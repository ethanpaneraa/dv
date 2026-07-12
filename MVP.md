# MVP

A terminal diff viewer for reviewing changes made by coding agents, in the spirit of
[hunk](https://github.com/modem-dev/hunk) but scoped down and built to our own taste.

## Scope decisions

- **Language/stack**: Rust, chosen for speed and ecosystem fit — the reference terminal
  diff/git tools (`delta`, `difftastic`, `bat`, `gitui`) are all Rust, so mature crates
  and prior art exist for every hard part.
- **Crates**: `ratatui` (TUI), `syntect` (syntax highlighting, same crate `bat`/`delta`
  use), `similar` (diffing, currently unused — reserved for future intra-line diffing),
  `crossterm` (terminal backend), `anyhow` (error handling).
- **Diff source**: shell out to `git diff` / `git diff --staged`. No libgit2 binding,
  no non-Git VCS support.
- **View mode**: unified only. Side-by-side is a real feature, not a toggle, and is
  deferred to keep the layout problem (column widths, terminal-width breakpoints) out
  of the MVP.

## What's in the MVP

- `dv` — shows `git diff` (working tree) in a TUI. (Superseded by discovery + Home
  screen below — kept here as the original scope this built on.)
- `dv --staged` — shows `git diff --staged`.
- File sidebar + unified diff pane.
- Syntax highlighting per file, based on extension.
- Added/removed background tinting on top of syntax colors.
- Dual line-number gutter (old/new).
- Keyboard navigation: `j`/`k` (or arrows) scroll, `d`/`u` page, `n`/`p` (or Tab/arrows)
  switch files, `g` jump to top, `q`/Esc quit.
- Clean fallback: prints "No changes to show." and exits without entering the TUI when
  the diff is empty.
- Panic hook that restores the terminal (raw mode off, alternate screen closed) before
  printing the panic, so a crash doesn't leave the terminal broken.

## Shipped since MVP

- **Multi-project discovery** — `dv`, with no flags, discovers what there is to review:
  the current directory (if it's itself a git repo) plus its immediate git-repo
  subdirectories, diffing each and keeping only the ones with actual changes. This is
  the flagship differentiator identified against `hunk` and typical terminal diff tools
  (`delta`, `difftastic`), none of which track more than one repo at a time — confirmed
  by checking `hunk`'s README, feature table, and issue tracker directly. `--scan <dir>`
  overrides which directory gets discovered (defaults to cwd); an explicit `dv <path>`
  bypasses discovery entirely for scripting/pager use.
- **Home screen, not a CLI flag** — this went through two iterations. First, a permanent
  Projects sidebar column: rejected, it ate width from the diff pane and split navigation
  across two key models. Second, a floating command-palette overlay (double-tap Space):
  better, but still modal-on-top-of-content rather than an actual app. Landed on an
  nvim-dashboard-style **full-page Home screen** instead: `dv` always opens here first
  (an ASCII logo, a fuzzy-filterable project list, a footer) regardless of how many
  projects were found — including exactly one, for consistency — and Enter opens the
  selected project into Files + Diff. Double-tapping Space from within a project returns
  to Home (full-page, not an overlay). Only `dv <path>` (an explicit path) skips Home,
  for scripting.
- **Startup performance** — measured with a 5-repo, ~600-changed-line-per-repo stress
  test (`time dv`, first draw to first keypress): 1.26s → 0.44s. Two fixes, found by
  instrumenting `main`/`App::new` with real timers rather than guessing:
  - Rendering (syntax highlighting via `syntect`) was eager — `App::new` highlighted
    every file in every discovered project before Home even had a chance to draw,
    even though Home never shows any of it. Now lazy: a project's files are only
    highlighted the first time it's actually opened (`home_confirm`/`next_project`/
    `prev_project`), cached after. The `Highlighter` itself (loading `syntect`'s
    default syntax/theme sets) is now lazily constructed too, on that same first use.
  - Loading each repo's diff (`git diff` subprocess spawn + parse) was sequential —
    5 repos meant 5 spawns back to back, ~480ms of the ~500ms total at that point.
    Now parallelized with `std::thread::scope` (no new dependency); wall time drops
    to roughly the single slowest repo's diff instead of the sum of all of them.
- **Home became a real dashboard, not a bare list** — feedback was that it read as a
  proof-of-concept (a single box with a filter and a flat list) rather than a designed
  app. Added: a stat line under the logo ("3 projects with changes • 13 files • +13
  -13"); a live **preview pane** next to the project list showing the highlighted
  project's files, each with its own `+N -M`; per-project `+N -M` in the list itself
  (via `diffmodel::file_stats`, a cheap line-count, no highlighting needed); and
  accent-colored key hints in the footer (key in blue-bold, description in gray)
  instead of flat gray text throughout.
- **Visual hierarchy between panes** — the Diff pane's border is now accent blue
  (`Color::Rgb(97, 175, 239)`) and the Files sidebar's is dim gray, since the Diff pane
  is always the primary, always-scrollable content and Files is a secondary nav rail.
  This is deliberately not a real focus-toggle (no mode where arrow keys stop
  scrolling the diff) — `n`/`p`/arrows and `j`/`k` were never actually ambiguous, so a
  toggle would add a mode without fixing a real conflict. If that changes (e.g. Files
  needs its own independent up/down navigation), revisit as a genuine `Focus` enum.
- **Watch mode** — the actual reason this tool exists: reviewing agents while they
  work, without quitting and relaunching every time a file changes. A background
  thread (`src/watch.rs`) re-diffs every loaded project root every 2s and sends
  changes over an `mpsc` channel (no new dependency — `std::thread` + polling, not a
  filesystem-events crate). The event loop switched from a blocking `event::read()` to
  `event::poll(200ms)` so it wakes up on its own to drain the channel even with no
  keypresses. Verified live: edited a tracked file from outside the running session,
  watched the diff pane update within ~2s with no input. Two deliberate constraints to
  keep it from being disorienting mid-review: a project whose changes get committed
  away keeps showing its last-known content instead of vanishing from the list, and
  the watcher only re-diffs the roots discovered at startup — it doesn't add newly
  appearing repos or remove ones that disappear.
- **Parser robustness** — added real unit tests for `diffmodel::parse` (previously a
  documented gap), built from *real* `git diff` output captured against scratch repos
  rather than hand-written fixtures: new files, deletions (path correctly taken from
  the `a/` side since `b/` is `/dev/null`), pure renames (zero hunks, similarity
  index 100%), renames with content changes, "no newline at end of file" markers,
  and true binary file diffs (`Binary files ... differ`, no hunks, no crash). All 9
  passed on the first real run — no bugs found, but the behavior is now locked in
  against regressions.
- **`rust-toolchain.toml` pinned** to 1.97.0, so a future clone doesn't hit the same
  `edition2024`/Cargo-1.83 wall we hit at project start.
- **Arrow-key drill-in/out on Home** — `Right`/`Enter` on a highlighted project opens
  it into the Diff view (file-manager style: a project "contains" its diff); `Left`
  from the Diff view returns to Home. This took over `Left` in the Diff view, which
  previously meant "previous file" — that's now `p`/Shift-Tab only, freeing `Left` for
  the drill-out semantics. `Right`/`Tab`/`n` are unchanged (still "next file") since
  there's nothing further right to drill into once you're already in the Diff view.
- **Fixed: files scrolling out of view looked broken** — the Files sidebar and
  Projects list rebuilt a fresh `ListState::default()` on every single frame, so
  ratatui had no memory of where the viewport was scrolled to. The moment the
  selection crossed the visible window, it recomputed the scroll from a blank slate
  and jumped straight to placing the selection at the bottom edge — looking like every
  file above had vanished, rather than scrolling incrementally. Fixed by persisting
  `ListState` in `App` (`files_list_state`, `project_list_state`) across frames.
  Verified in a 14-row terminal with 19 changed files: stepping through now scrolls
  one line at a time in both directions, reaching both ends of the list correctly.
- **Fixed: no way to discover file-navigation keys** — the Diff screen's footer showed
  project-switching hints (`{ }`, `space space`) but never mentioned `n`/`p`, the
  actual keys that move between files in the sidebar. `{`/`}` are for switching
  *projects* — with only one project loaded they're correctly no-ops, which read as
  "these keys don't do anything" rather than "there's nothing to switch to." Footer
  now always shows `n/p: file`.
- **Removed `{`/`}` project-switching entirely** — even after the footer hint fix
  above, this caused a second, worse confusion: with multiple projects loaded,
  pressing `}` silently swapped the *entire* Files list to a different project with no
  visual cue that a switch had happened, which read as "my files just vanished" rather
  than "I switched context." Two confusing incidents from the same feature was enough
  to conclude it wasn't earning its complexity. The only way to switch projects now is
  double-tap `Space` → Home screen → pick one explicitly — deliberate, and it shows you
  exactly what you're picking before you commit to it.

## Roadmap (next, in priority order)

1. **Side-by-side / split view** — real readability upgrade for larger changes, deferred
   from MVP because of the added layout complexity (column widths, terminal-width
   breakpoints).
2. **Watch mode refinements** — rediscover new repos appearing under the scan root
   without a restart; surface a "committed" indicator on projects the watcher notices
   went to zero changes, instead of silently freezing their last content.

## Explicitly deferred (not on the near-term roadmap)

- Jujutsu / Sapling VCS support
- Raw patch input via stdin (`dv patch -`)
- Agent annotation / live-collaboration system
- Custom theming / config file
- Git pager replacement mode (`git diff` → `dv` as `core.pager`)

## Known MVP limitations

- Syntax highlighting runs per-file across hunks in order, without the surrounding
  file context for lines that were removed or that sit before the first hunk. Multi-line
  constructs (block comments, long strings) spanning a skipped region between hunks may
  highlight slightly wrong. Acceptable for now; fixing it means diffing against full
  old/new file content instead of raw hunk lines.
- Highlighted output is rebuilt once per file when the app loads, not incrementally —
  fine at typical diff sizes, would need revisiting for huge diffs.
