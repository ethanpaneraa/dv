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

- `dv` — shows `git diff` (working tree) in a TUI.
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

- **Multi-project view** — `dv --scan <dir>` discovers immediate git-repo subdirectories
  of `<dir>`, computes a diff for each, and only surfaces projects that currently have
  changes. This is the flagship differentiator identified against `hunk` and typical
  terminal diff tools (`delta`, `difftastic`), none of which track more than one repo at
  a time — confirmed by checking `hunk`'s README, feature table, and issue tracker
  directly.
- **Command palette for project switching** — the first cut of multi-project view used
  a permanent Projects sidebar column; that ate width from the diff pane (the thing that
  actually matters) and split navigation across two different key models. Replaced with
  an app-like pattern instead: single-repo layout (Files + Diff) stays the default view
  regardless of project count, and double-tapping Space (within ~350ms) pops a centered,
  fuzzy-filterable switcher — type to narrow, arrows to move, Enter to jump, Esc to
  cancel. `{`/`}` still cycle projects directly for fast switching without opening the
  palette. A footer hint bar shows the current project name (when >1 loaded) and the
  `space space` / `q` hints; it disappears in single-repo mode where there's nothing to
  switch to.

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
