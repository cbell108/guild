Rules for this unattended overnight session:
- Read CLAUDE.md, docs/API.md and docs/DECISIONS.md first. Work through your lane in TASKS.md top to bottom (MUST before STRETCH).
- After each task: cargo fmt --all; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace; commit on your branch with a Conventional Commit naming the task ID; tick the box in TASKS.md; then start the next task.
- Edit only your lane's crates/paths. FROZEN for everyone: crates/core, crates/api, docs/API.md, the JSON shapes in static/mock/, and the public signatures listed as frozen in crates/store/src/lib.rs, crates/recommend/src/lib.rs and crates/llm/src/lib.rs (implement their bodies in your own lane; never change a signature). If you need such a change, append it under your lane in NEEDS.md and work around it.
- Your branch has the other lanes' crates as stubs (store methods return StoreError::NotImplemented; recommend/llm return placeholders or NotImplemented). Test your own code with pure functions and stub-tolerant paths; integration happens at merge.
- Dependencies: everything expected is already declared in the root Cargo.toml. Prefer those. If you truly need a new crate, add it only to your own crate's Cargo.toml with an exact version. Cargo's "unused dependency" warnings for pre-declared deps are expected; do not remove dependencies.
- Use runtime sqlx::query() only (no query! macros). No unwrap/expect outside tests. Every pub item gets a doc comment.
- If a task still fails after 3 attempts, write what you tried under your lane in BLOCKED.md, leave its box unticked, and move on.
- Do not push, do not read or edit .env, do not add secrets, do not delete other branches or worktrees.
- Do not stop until every task in your lane is ticked or recorded in BLOCKED.md.
