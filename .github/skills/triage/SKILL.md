---
name: triage
description: Use when the user wants to triage a task/bug/idea into a well-formed GitHub issue for a future agent to implement. Discusses the task, investigates what needs doing and where, classifies it, and creates a GitHub issue only after the user accepts. Trigger phrases include "triage", "triage this", "create an issue", "file an issue", "turn this into a task", "raise a ticket".
---

# Triage

Turn a rough task, bug, or idea into a **well-formed GitHub issue** that a *different agent* can implement later with no prior conversation context. The skill discusses the task, investigates the codebase to find what needs doing and where, classifies it, and — **only after the user explicitly accepts** — creates the issue in this project's GitHub repo.

## Principles

- **Discuss first, create last.** Never run `gh issue create` until the user has reviewed the drafted issue and explicitly accepted it.
- **The issue is for another agent, at another time.** Write it as a self-contained task: enough context, file references, and acceptance criteria that an agent with zero conversation history can pick it up and implement it.
- **Ask for missing details.** Do not guess when scope, expected behavior, or acceptance criteria are ambiguous.

## Input

The user provides a task, bug report, or idea in their request. It may be vague (e.g., "the terminal pager mangles curved table borders on narrow widths") or a feature ask.

## Steps

### 1. Understand and discuss the task

1. Restate the task in your own words to confirm understanding.
2. Determine the **type of work**: is it a bug, a new feature, a refactor, etc.? (This informs the category later.)
3. Ask the user any clarifying questions needed to write a complete task. Use the `ask_user` tool. Typical gaps to probe:
   - Expected vs. actual behavior (for bugs)
   - Which **output surface(s)** are affected (Tauri/wry GUI, terminal pager, nvim live preview)
   - Scope boundaries (what is explicitly *out of scope*)
   - Acceptance criteria / definition of done
   - Any constraints (performance, backwards compatibility, the never-break plugin/theme contracts, specific crates to touch or avoid)
   - What tests should add/validate the fix (triaged issues include their tests as part of the fix)
4. Do **not** proceed to investigation until the task is clear enough to investigate.

### 2. Investigate "what needs to be done, and where" (via subagent)

Launch a fresh **general-purpose** subagent (via the Task tool) with **no prior conversation context** — build the prompt from scratch. This keeps the main conversation clean.

The subagent prompt must include:
- The **task description** as clarified in step 1
- The **task type** (bug/feature/refactor/etc.)
- The affected **output surface(s)** if known
- The instruction to read the root `AGENTS.md` for the monorepo layout, conventions, the "Architecture reality" section, and the "Where to extend" table before touching anything
- The **investigation goals** below

The subagent is responsible for:
1. Reading the root `AGENTS.md` for crate layout, conventions, contracts, and the "Architecture reality" / "Where to extend" guidance.
2. Using grep/glob/view to locate the **specific files, crates, structs, traits, and functions** that must change or be added. **Capture short verbatim code snippets** (with `file:line` refs) of the current relevant code — for bugs the exact offending lines, for features/refactors the code that will be extended or replaced — so the drafted issue can embed them.
3. Identifying the **crate(s)** involved (e.g., `mdview-core`, `mdview-theme`, `mdview-ext-*`, `mdview-render-*`, `apps/mdview`, `sidecar/`).
4. Confirming the **root cause** (for bugs) by tracing the actual code — not guessing. Remember the app is plain `wry` + `tao` (not Tauri), and most GUI features live as embedded JS/CSS in `apps/mdview/src/render.rs`.
5. Sketching a **proposed approach** consistent with existing patterns (`MdViewExtension` trait, `Theme` contract, `canonical_lang` arms, preset registration, etc.) and **without breaking the plugin or theme contracts**.
6. Noting **testing implications**: which test targets map to the changed source (in-crate `#[cfg(test)]` modules, crate `tests/` integration tests, or `tests/e2e` Playwright), which **existing** tests will break and must be updated, and what **new** test cases would add/validate the fix (specific `#[test] fn` names and the behavior each asserts).
7. Flagging any ambiguity or missing information the user still needs to resolve.

The subagent must **return a structured report** containing:
- `affected` — list of `{file, symbol, why}` entries (files/crates/structs/functions to change or add)
- `codeSnippets` — short verbatim excerpts of the current relevant code, each with a `file:line` reference and the language for fencing (for bugs, the exact offending lines; otherwise the code to be extended/replaced)
- `crates` — the crate(s) involved
- `rootCause` — for bugs, the confirmed root cause with file:line references (or "n/a")
- `approach` — the proposed implementation approach
- `testing` — test files/impact, the exact `cargo test` selector(s), and a concrete **regression test snippet** whose assertions pin *this specific* fixed behavior (see Testing requirements below)
- `openQuestions` — anything still unclear

If the subagent returns `openQuestions`, resolve them with the user (via `ask_user`) before drafting the issue.

### 3. Classify the task

Assign **one or more categories** from the fixed set below (these map 1:1 to GitHub labels):

| Category      | Use case                                        |
|---------------|-------------------------------------------------|
| `bug`         | Something is broken or behaves incorrectly      |
| `feature`     | New capability                                  |
| `refactor`    | Restructuring without behavior change           |
| `test`        | Adding or updating tests                        |
| `docs`        | Documentation changes                           |
| `chore`       | Maintenance, dependencies, cleanup              |
| `performance` | Speed/memory/efficiency improvements            |
| `tech-debt`   | Paying down accumulated shortcuts               |

Pick the categories that genuinely apply (usually one primary, occasionally a secondary such as `bug` + `tech-debt`).

### 4. Draft the issue and present it for acceptance

1. Compose the issue **title** in Conventional Commit style: `<type>(scope): <short description>` (e.g., `fix(pager): curved table borders break at narrow widths`). Use the crate as the scope where it helps.
2. Compose the issue **body** with these sections (omit a section only if truly not applicable):
   - **Summary** — one or two sentences.
   - **Context / Background** — why this matters; the reported symptom or motivation; the affected output surface(s). **Embed the relevant current code** as a fenced snippet (from the subagent's `codeSnippets`) with a `file:line` caption whenever feasible, so the future agent sees exactly what code the task refers to.
   - **Affected files & crates** — bulleted list from the subagent's `affected` + `crates`, with `file` -> `symbol` -> reason. Reference the "Where to extend" / "Architecture reality" notes in `AGENTS.md` where relevant.
   - **Proposed approach** — the subagent's `approach`, plus root cause for bugs. Call out any contract that must **not** break.
   - **Acceptance criteria** — a checklist of concrete, verifiable outcomes.
   - **Testing requirements** — **always required.** Spell out the concrete test changes needed to add and/or validate the fix, so a future agent (or the user) can verify the change is done. Triaged issues are expected to include their tests as part of the fix — do **not** add a "tests only when explicitly requested" caveat here:
     - The exact test target(s) that map to the changed source (in-crate `#[cfg(test)]` module, crate `tests/*.rs`, or `tests/e2e`).
     - **Existing tests that will break** and must be updated (name them, and say how).
     - **New test cases** that add/validate the fix — proposed `#[test] fn <name>` names and the specific behavior/assertion each covers. Include a concrete **regression test code snippet** (fenced) that the future agent can drop in.
     - **Pin the fix, not incidental state.** Each new test's assertions must fail today (before the fix) and pass only once *this specific* change is made, and must target the fixed behavior narrowly — assert the exact output/value that changes, not a broad snapshot or unrelated surrounding state — so the test verifies the fix is done and does not silently pass (or break) because of unrelated future changes. Prefer a focused assertion on the changed symbol over a whole-document snapshot.
     - **GUI-affecting changes need a visual/DOM check.** When the change alters the wry webview output, add or extend a `tests/e2e` Playwright test that asserts the rendered DOM (selector visible) and, where the change is visual, a computed-style assertion (e.g. token color differs from plain text) plus the existing full-page screenshot size check — string-only HTML assertions are not sufficient to prove it renders.
     - The exact command to run for validation (e.g., `cargo test -p mdview-ext-highlight canonical_lang`, or `cargo test --workspace`; `cd tests/e2e && bun run test:e2e` for GUI). Note that `cargo fmt` and `cargo clippy -- -D warnings` must also pass.
   - **Out of scope** — what this task must not touch (e.g., non-goals in `AGENTS.md`).
   - **Do not** add a `Categories:` footer line to the issue body. Track the chosen categories
     separately (they are passed to the create script as `-Label` flags), not inside the body.
   - **Dependencies / sub-issues** — if this task depends on, or is a child of, another issue
     (existing or one filed in the same batch), note the parent explicitly (e.g., `Depends on #NN`)
     so it can be linked as a **sub-issue** of that parent after creation (see step 7).
3. Write the draft to `tmp/triage-issue.md` (git-ignored) so the user can edit it directly.

### 5. Validate the draft with a rubber-duck agent — always

**Always run this step before presenting the draft to the user.** After writing
`tmp/triage-issue.md`, launch a **rubber-duck** agent (via the Task tool) to
review the drafted issue for correctness and completeness. Give it the full
draft text plus the original task and the subagent's investigation report, and
ask it to check specifically that:

- The technical claims (file paths, symbols, triggers, root cause) are accurate
  and not fabricated.
- The **testing requirements actually pin *this* fix** — the proposed
  assertions would fail before the change and pass only after it, and target the
  changed behavior narrowly rather than a broad snapshot or unrelated state.
- The acceptance criteria are concrete and verifiable, and the scope boundaries
  (out-of-scope, contracts that must not break) are correct.
- Nothing important is missing and nothing contradicts `AGENTS.md`.

Address the rubber-duck's findings by revising `tmp/triage-issue.md` before
moving on. Only proceed once the draft holds up to review.

### 6. Present the reviewed draft for acceptance

Show a summary in chat including the proposed **title** and **categories/labels**,
then **ask the user to accept, edit, or reject** using the `ask_user` tool
(accept / edit / reject). If they choose "edit", let them edit
`tmp/triage-issue.md` (and/or adjust categories) and wait for confirmation, then
re-read the file.

### 7. Create the GitHub issue — only if accepted

**Only run this step if the user accepted in step 6.** If rejected, delete `tmp/triage-issue.md` and stop.

Use the bundled helper script — it derives the repo root, ensures each label exists (creating any that are missing), extracts the title from the first `# ` heading, writes the body to a git-ignored temp file, creates the issue assigned to `@me`, and prints the title and URL:

```ps1
pwsh -NoProfile -ExecutionPolicy Bypass `
  -File .github/skills/triage/scripts/new-issue.ps1 `
  -Draft tmp/triage-issue.md -Label <cat1> -Label <cat2>
```

Pass one `-Label` per chosen category. Then report the created issue URL to the user.

**Link dependent issues as sub-issues.** If the draft declared a parent (a `Depends on #NN` issue,
or a broader item this was split from), link the newly created issue as a **sub-issue** of that
parent using the GitHub sub-issues API. Note the API needs the child's integer database `id` (not
its number) and must be passed as an integer with `gh api -F`:

```ps1
$childId = gh api repos/<owner>/<repo>/issues/<child_number> --jq ".id"
gh api --method POST repos/<owner>/<repo>/issues/<parent_number>/sub_issues -F sub_issue_id=$childId
```

Verify the parent's `sub_issues_summary.total` incremented. Then report the created issue URL to the
user.

### 8. Cleanup

1. Delete `tmp/triage-issue.md` and any temp body file.
2. Show a short summary: issue URL, title, and assigned categories/labels.
