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

## Explicitly deferred (not MVP)

- Side-by-side / split view
- Watch mode (auto-reload on file/git changes)
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
