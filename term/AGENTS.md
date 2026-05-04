# TERM KNOWLEDGE BASE

## Scope

`term/` contains terminal emulation core behavior, including screen state, escape sequence handling, selection, and PTY-facing terminal semantics.

## Where To Look

- `term/src/terminal.rs` - main terminal state and event handling.
- `term/src/terminalstate/` - detailed state transitions.
- `term/src/screen.rs` - screen buffer behavior.
- `term/src/selection.rs` - selection and copy behavior.
- `term/src/escape/` - escape sequence handling when present.

## Practical Rules

- Treat terminal semantics as compatibility-sensitive.
- Prefer existing parser and state helpers over ad hoc escape handling.
- Keep GUI assumptions out of terminal core.
- Preserve screen buffer invariants and add focused tests for behavior changes.
- Keep normal-screen and alternate-screen behavior distinct; wheel scrolling and inline AI status must not corrupt terminal state.
- Preserve selection and cursor invariants that GUI overlays and inline assistant status depend on.

## Verification

- Compile checks: run `make check`.
- Terminal logic changes: run `make test` or the narrow cargo nextest filter for the affected behavior.
- Cross-crate behavior: also inspect `mux/AGENTS.md` and `kaku-gui/AGENTS.md`.

## Cross-References

- `../mux/AGENTS.md` - owns panes and terminal instances.
- `../kaku-gui/AGENTS.md` - renders terminal surfaces.
- `../termwiz/AGENTS.md` - terminal UI primitives.
