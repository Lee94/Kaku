# TERMWIZ KNOWLEDGE BASE

## Scope

`termwiz/` contains terminal UI primitives, widgets, input handling helpers, and rendering utilities used by Kaku command-line and overlay flows.

## Where To Look

- `termwiz/src/` - primary library code.
- `termwiz/src/widgets/` - reusable widgets.
- `termwiz/src/lineedit/` - line editing behavior.
- `termwiz/src/render/` - rendering helpers.

## Practical Rules

- Keep primitives generic; do not leak Kaku-specific policy into reusable widgets.
- Prefer existing widget contracts over one-off UI behavior.
- Preserve keyboard navigation and terminal rendering compatibility.
- Add targeted tests when widget state or input behavior changes.
- Reuse existing form, theme, and component primitives for both config TUI and AI config TUI.
- Keep debounce and terminal repaint behavior in shared primitives rather than duplicating it in feature-specific TUI code.

## Verification

- Compile checks: run `make check`.
- UI primitive logic changes: run the narrow affected tests, then `make test` if behavior is shared.
- Overlay or GUI consumers: inspect `kaku-gui/AGENTS.md` and verify the consuming flow.

## Cross-References

- `../kaku-gui/AGENTS.md` - GUI and overlay consumers.
- `../term/AGENTS.md` - terminal core behavior.
