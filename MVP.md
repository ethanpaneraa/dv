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
  to Home (full-page, not an overlay); `{`/`}` still cycle projects directly without
  leaving the diff. Only `dv <path>` (an explicit path) skips Home, for scripting.
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

## Roadmap (next, in priority order)

1. **Watch mode** — auto-reload as files/git state change, built on top of the
   multi-project loader above (re-poll each loaded project root, not just cwd).
2. **Parser robustness** — no test coverage yet for `diffmodel::parse` against renames,
   deletions, new binary files, or "no newline at end of file." Agents produce all of
   these; a wrong render is worse than a missing feature.
3. **Side-by-side / split view** — real readability upgrade for larger changes, deferred
   from MVP because of the added layout complexity (column widths, terminal-width
   breakpoints).
4. **Polish** — `rust-toolchain.toml` pin.

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
