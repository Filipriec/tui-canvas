# Aider Instructions

## General Rules
- Only modify files that I explicitly add with `/add`.
- If a prompt mentions multiple files, **ignore all files except the ones I have added**.
- Do not create, edit, or delete any files unless they are explicitly added.
- Keep all other files exactly as they are, even if the prompt suggests changes.
- Never move logic into or out of files that are not explicitly added.
- If a prompt suggests changes to multiple files, apply **only the subset of changes** that belong to the added file(s).
- If a change requires touching other files, ignore them, if they were not manually added.

## Coding Style
- Follow Rust 2021 edition idioms.
- No logic in `mod.rs` files (only exports/routing).
- Always update or create tests **only if the test file is explicitly added**.
- Do not think, only apply changes from the prompt
