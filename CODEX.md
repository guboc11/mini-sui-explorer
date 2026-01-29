# CODEX.md

# Prompt-Based Workflow

- For each prompt, implement with the minimum necessary code changes.
- If large changes are required, brief the plan first and propose splitting the work.
- For each prompt, make exactly one git commit.
- Commit message must include the prompt in English, even if the original prompt was in Korean.
- Commit message format: "\[PRMPT\] {COMMIT_TYPE}: {PROMPT_MESSAGE}"

# Basic Rules

## Core Principles

- Favor clarity over cleverness.
- Keep changes small and reviewable.
- Optimize for local developer speed.
- Prefer explicitness to guesswork.

## Style & Conventions

- Use existing project patterns first.
- Keep functions focused; avoid long files.
- Prefer early returns over deep nesting.
- Avoid non-ASCII unless already used in file.
- Comments only when behavior is non-obvious.

## Git Hygiene

- Do not modify unrelated files.
- Keep commits scoped to one intent.
- Never amend unless explicitly asked.

## Testing & Safety

- Add or update tests for behavioral changes.
- Avoid destructive commands unless requested.
- Ask before making breaking changes.

## Documentation

- Update README or inline docs when needed.
- Mention new env vars, configs, or scripts.

## Workflow

- Use `rg` for searching.
- Prefer `apply_patch` for small edits.
- Summarize changes and suggest next steps.

## Work Plan

- Maintain a granular, step-by-step schedule in `WORK_PLAN.md`.
- Organize the plan by date and a goal-focused title.
- Under each date, add one subheading that groups tasks.
- Keep tasks extremely small, concrete, and executable.
- Update the plan whenever scope or rules change.

## Questions & Assumptions

- Ask if requirements are ambiguous.
- State assumptions before implementation.
