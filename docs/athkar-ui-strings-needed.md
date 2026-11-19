# Athkar UI string keys needed (Forge → Quill)

Phase 4 shipped with existing patterns + corpus data (category names are
corpus content, not UI copy). These NEW functional labels need Quill's
AR/EN catalog entries (`docs/athkar-ui-strings.md` was not yet landed at
implementation time — consume and replace when it arrives):

| Key | Current EN | Where | Notes |
|---|---|---|---|
| `athkar.context.after_prayer` | "After-prayer athkar" | context chip + card label | Morning/Evening/Sleep reuse the scheduler's existing window labels |
| `athkar.auto` | "Auto" | context switcher chip | re-follow the prayer-window context |
| `athkar.reset` | "Reset" | athkar view action | clears the shown context's day only |

Numeric progress ("3/24", "2/3") and the completion glyph (✓) are
language-neutral. The back action reuses the existing "إغلاق".
