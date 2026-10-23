---
name: triage
description: Use when the user wants to triage a task/bug/idea into a well-formed GitHub issue for a future agent to implement. Discusses the task, investigates what needs doing and where, classifies it, and creates a GitHub issue only after the user accepts. Trigger phrases include "triage", "triage this", "create an issue", "file an issue", "turn this into a task", "raise a ticket".
---

# Triage

Turn a rough task, bug, or idea into a **well-formed GitHub issue** that a *different agent* can implement later with no prior conversation context. The skill discusses the task, investigates the codebase to find what needs doing and where, classifies it, and — **only after the user explicitly accepts** — creates the issue in this project's GitHub repo.

## Principles

- **Discuss first, create last.** Never run `gh issue create` until the user has reviewed and explicitly accepted
  the whole plan, including every parent and child draft.
- **The issue is for another agent, at another time.** Write it as a self-contained task: enough context, file references, and acceptance criteria that an agent with zero conversation history can pick it up and implement it.
- **Anchor to stable identifiers, not coordinates.** An issue is implemented at an unknown later time against a changed tree. Reference symbols, string literals, config keys, and search patterns — never line numbers, occurrence counts, or verbatim multi-line snapshots that the codebase will invalidate. A frozen inventory goes stale *and* misleads: the implementing agent trusts it as exhaustive and skips rediscovering occurrences added after the issue was filed. The issue points the way; the implementing agent re-discovers the exact scope.
- **Match detail to task type.** For a narrow **bug**, the exact offending lines *are* the deliverable — a tight snippet of the actual defect is worth freezing. For **features / refactors / renames / broad sweeps**, staleness dominates: give a discovery recipe (search patterns + stable anchors) and the contract/invariant rules, not an enumerated file list.
- **Ask for missing details.** Do not guess when scope, expected behavior, or acceptance criteria are ambiguous.
- **Check what already exists.** Search the repo's open and closed issues before filing — surface duplicates to the user, and link genuinely related issues rather than creating disconnected ones.

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
2. Using grep/glob/view to locate the work. **Scale the output to the task type** (see the "Match detail to task type" principle):
   - **Narrow bug:** capture the exact offending lines as a short verbatim snippet with a `file:line` ref — the defect *is* the deliverable and won't move much.
   - **Feature / refactor / rename / broad sweep:** do **not** enumerate every hit. Identify the **distinct surfaces/categories** the work touches, give **one representative entry point** per surface (as a stable anchor — a symbol, string literal, or config key), and record the **grep/glob patterns** that locate the rest. Prefer "find the remaining refs via `<pattern>`" over a frozen file list or an occurrence count.
3. Identifying the **crate(s)** involved (e.g., `mdview-core`, `mdview-theme`, `mdview-ext-*`, `mdview-render-*`, `apps/mdview`, `sidecar/`).
4. Confirming the **root cause** (for bugs) by tracing the actual code — not guessing. Remember the app is plain `wry` + `tao` (not Tauri), and most GUI features live as embedded JS/CSS in `apps/mdview/src/render.rs`.
5. Sketching a **proposed approach** consistent with existing patterns (`MdViewExtension` trait, `Theme` contract, `canonical_lang` arms, preset registration, etc.) and **without breaking the plugin or theme contracts**.
6. Noting **testing implications**: which test targets map to the changed source (in-crate `#[cfg(test)]` modules, crate `tests/` integration tests, or `tests/e2e` Playwright), which **existing** tests will break and must be updated, and what **new** test cases would add/validate the fix (specific `#[test] fn` names and the behavior each asserts).
7. Flagging any ambiguity or missing information the user still needs to resolve.

The subagent must **return a structured report** containing:
- `surfaces` — the distinct categories of change the task touches (for a narrow bug this is usually one); for each, a short reason and its contract/invariant rules if any
- `affected` — representative entry points as `{file, symbol, why}`. For bugs, the specific site(s). For features/refactors/renames, **one canonical example per surface**, not an exhaustive inventory
- `searchStrategy` — the grep/glob patterns and stable anchors (symbol names, string literals, config keys, `canonical_lang` arms) that let the implementing agent **re-discover the full scope** later. This replaces a frozen file enumeration
- `codeSnippet` — for a **narrow bug only**, a short verbatim excerpt of the offending lines with a `file:line` ref and language for fencing. Omit for features/refactors/renames (reference the pattern, not the lines)
- `crates` — the crate(s) involved
- `rootCause` — for bugs, the confirmed root cause with file:line references (or "n/a")
- `approach` — the proposed implementation approach
- `slices` — when needed, proposed bounded implementation children, coverage, dependencies, and each child's source
  complexity/rationale; reassess nested slices using step 4
- `sourceComplexity` — low, medium, high, or unresolved, with production-code evidence using step 4's rules
- `testing` — test files/impact, the exact `cargo test` selector(s), and a concrete **regression test snippet** whose assertions pin *this specific* fixed behavior (see Testing requirements below)
- `openQuestions` — anything still unclear

Reference stable anchors (symbols, string literals, config keys), **never line numbers or occurrence counts** in the enumerated scope — the issue is implemented later against a moved tree.

If the subagent returns `openQuestions`, resolve them with the user (via `ask_user`) before drafting the issue.

### 3. Search for existing duplicate and related issues

**Always run this before classifying** — never file an issue without first checking what already
exists. The goal is to avoid duplicates and to discover issues that should be linked.

1. Build search terms from the task and the step-2 investigation: key symbols, file/crate names,
   error strings, feature names, and the affected output surface(s).
2. Query the repo's issues — **open and closed** — for candidates. Derive `<owner>/<repo>` from
   `gh repo view --json nameWithOwner --jq .nameWithOwner`:
   ```ps1
   gh search issues --repo <owner>/<repo> "<keywords>" --state all --limit 30 --json number,title,state,url
   # try several keyword combinations; also list within the repo:
   gh issue list --search "<keywords> in:title,body" --state all --limit 30 --json number,title,state,url
   ```
3. Classify each candidate found:
   - **Duplicate** — same underlying task: same root cause (bugs) or the same requested capability
     (features). A closed-and-fixed duplicate still counts.
   - **Related** — a distinct task that overlaps in area, shares files/root cause, or is a
     parent/child (containment) or prerequisite of this one.
4. **If a likely duplicate exists:** stop and inform the user with the `ask_user` tool. Show the
   duplicate's `#number`, title, state, and URL, and ask how to proceed — **abort** (don't file),
   **file anyway** (genuinely distinct; explain why in the body), or **comment on the existing
   issue** instead of filing a new one. Do not create a new issue unless the user chooses to.
5. **If related issues exist:** record each as `#number` + a one-line reason. These are carried into
   the draft (step 5) and linked after creation (step 8):
   - A **parent/child** containment relationship uses native GitHub **sub-issues**.
   - A **blocked-by** dependency records implementation order separately; it does not imply parent containment.
   - A plain **association** is linked by mentioning `#number` in the issue body, which creates a
     GitHub cross-reference relationship automatically.

### 4. Classify the task and assess source complexity

Assign **one or more categories** from the fixed set below (these map 1:1 to GitHub labels):

| Category      | Use case                                        |
|---------------|-------------------------------------------------|
| `bug`         | Something is broken or behaves incorrectly      |
| `chore`       | Maintenance, dependencies, cleanup              |
| `docs`        | Documentation changes                           |
| `feature`     | New capability                                  |
| `performance` | Speed/memory/efficiency improvements            |
| `refactor`    | Restructuring without behavior change           |
| `spike`       | Investigation/exploration only — no code or docs changes; produces findings (and usually a follow-up issue) |
| `test`        | Adding or updating tests                        |
| `tech-debt`   | Paying down accumulated shortcuts               |

Pick the categories that genuinely apply (usually one primary, occasionally a secondary such as
`bug` + `tech-debt`). Apply **`spike`** when the task is purely investigation/exploration whose
deliverable is *findings* — it must not change any code or docs. A spike is typically the only
category on such an issue; do not combine it with `bug`/`feature`/`refactor` (file a separate
follow-up issue for the resulting work).

Assess **source implementation complexity** from actual production code: behavioral scope, component coupling,
interfaces, state/lifecycle changes, migrations/invariants, and unresolved design. Exclude test additions/updates,
fixtures, harnesses, matrix breadth, coverage, test runtime, and test-writing effort.
Keep required tests and validation attached to each implementation slice; do not omit them or split off test-only work
because it is large. Pure docs, tests, and spikes are not high source complexity merely because they are extensive.

- **Low:** a localized, well-understood behavioral change.
- **Medium:** one coherent bounded outcome across a small connected set of components, with clear contracts and
  manageable invariants.
- **Above medium:** never file as a single implementation task. Draft a **tracking-only parent**, explicitly not an
  implementation assignment, and non-overlapping, independently verifiable low/medium children. Reassess every child
  recursively; any higher-complexity child becomes another tracking-only parent.

Do not use line/file-count or time quotas. If uncertainty prevents a bounded estimate, investigate or clarify with the
user rather than treating unknown as low. Preserve full request coverage and real dependencies, allowing independent
children to run in parallel. Repeat step 3 for **each proposed child** and reuse suitable existing issues instead of
duplicating them. Parent containment and blocked-by dependencies remain separate.

### 5. Draft the issue and present it for acceptance

1. Compose the issue **title** in Conventional Commit style: `<type>(scope): <short description>` (e.g., `fix(pager): curved table borders break at narrow widths`). Use the crate as the scope where it helps.
2. Compose the issue **body** with these sections (omit a section only if truly not applicable):
   - **Summary** — one or two sentences.
   - **Context / Background** — why this matters; the reported symptom or motivation; the affected output surface(s). **For a narrow bug**, embed the offending code as a fenced snippet (from the subagent's `codeSnippet`) with a `file:line` caption so the future agent sees exactly what code the task refers to. **For a feature/refactor/rename**, describe the *pattern* and the surfaces involved — do not paste a frozen multi-line snapshot or an occurrence count that the tree will invalidate.
   - **Affected surfaces & crates** — for a narrow bug, the specific `file` -> `symbol` -> reason sites. For a feature/refactor/rename, the **surfaces/categories** (from the subagent's `surfaces`) with **one representative entry point each** plus the **discovery command** that finds the rest (from `searchStrategy`). Prefix this section with a standing note to the implementing agent: *"This is a starting map, not an exhaustive inventory — re-run the discovery searches below and treat their output as authoritative; occurrences may have been added since this issue was filed."* Reference the "Where to extend" / "Architecture reality" notes in `AGENTS.md` where relevant.
   - **Proposed approach** — the subagent's `approach`, plus root cause for bugs. Call out any contract that must **not** break.
   - **Source complexity** — each implementation leaf records `low` or `medium` and a concise production-code reason
     from step 4. A tracking-only parent instead states that it is not an implementation assignment and records scope,
     completion criteria, child coverage, and dependencies; its validation is the linked children's validation.
   - **Acceptance criteria** — a checklist of concrete, verifiable outcomes.
   - **Testing requirements** — **always required** for implementation leaves (except `spike` issues; see below). Spell out the concrete test changes needed to add and/or validate the fix, so a future agent (or the user) can verify the change is done. Triaged issues are expected to include their tests as part of the fix — do **not** add a "tests only when explicitly requested" caveat here:
     - **`spike` (investigation only) issues are exempt:** they change no code or docs, so there are no tests. Replace this section with a **Deliverables** section instead — the questions the investigation must answer, where findings are recorded, and the expected follow-up (usually a new issue capturing the resulting work). Do not fabricate test requirements for a spike.
     - The exact test target(s) that map to the changed source (in-crate `#[cfg(test)]` module, crate `tests/*.rs`, or `tests/e2e`).
     - **Existing tests that will break** and must be updated — name the test target and function, and say how. Reference tests by name, not line number.
     - **New test cases** that add/validate the fix — proposed `#[test] fn <name>` names and the specific behavior/assertion each covers. Include a concrete **regression test code snippet** (fenced) that the future agent can drop in.
     - **Pin the fix, not incidental state.** Each new test's assertions must fail today (before the fix) and pass only once *this specific* change is made, and must target the fixed behavior narrowly — assert the exact output/value that changes, not a broad snapshot or unrelated surrounding state — so the test verifies the fix is done and does not silently pass (or break) because of unrelated future changes. Prefer a focused assertion on the changed symbol over a whole-document snapshot.
     - **GUI-affecting changes need a visual/DOM check.** When the change alters the wry webview output, add or extend a `tests/e2e` Playwright test that asserts the rendered DOM (selector visible) and, where the change is visual, a computed-style assertion (e.g. token color differs from plain text) plus the existing full-page screenshot size check — string-only HTML assertions are not sufficient to prove it renders.
     - The exact command to run for validation (e.g., `cargo test -p mdview-ext-highlight canonical_lang`, or `cargo test --workspace`; `cd tests/e2e && bun run test:e2e` for GUI). Note that `cargo fmt` and `cargo clippy -- -D warnings` must also pass.
   - **Out of scope** — what this task must not touch (e.g., non-goals in `AGENTS.md`).
   - **Do not** add a `Categories:` footer line to the issue body. Track the chosen categories
     separately (they are passed to the create script as `-Label` flags), not inside the body.
   - **Related issues** — from step 3. List each as `Related to #NN — <one-line reason>`. Mentioning
     `#NN` in the body creates a GitHub cross-reference relationship automatically, so a future agent
     sees the connection. Use this for plain associations; keep parent containment and blocked-by dependencies in the
     next bullet.
   - **Dependencies / sub-issues** — identify parent containment separately from `Depends on #NN` prerequisites.
     Record planned and reused children, their category labels, and the native hierarchy. Every child, including one
     attached to an existing parent or nested recursively, needs the exact `sub-issue` label plus its categories.
     A parent receives this label only if it is itself another issue's child.
3. Write the main draft to `tmp/triage-issue.md` (git-ignored) so the user can edit it directly. For a split plan, use
   distinct `tmp\triage-<guid>-<slug>.md` child drafts. Link them from the parent without duplicating their bodies.

### 6. Validate the draft with a rubber-duck agent — always

**Always run this step before presenting the draft to the user.** After writing
`tmp/triage-issue.md`, launch a **rubber-duck** agent (via the Task tool) to
review the drafted issue for correctness and completeness. Give it the full
text of **all parent and child drafts**, reused issue details, the original task, and the investigation report, and
ask it to check specifically that:

- The technical claims (file paths, symbols, triggers, root cause) are accurate
  and not fabricated.
- For a feature/refactor/rename, the scope is expressed as a **discovery recipe
  (search patterns + stable anchors)**, not a frozen file/line inventory or
  occurrence count that will go stale before the issue is implemented.
- The **testing requirements actually pin *this* fix** — the proposed
  assertions would fail before the change and pass only after it, and target the
  changed behavior narrowly rather than a broad snapshot or unrelated state.
- The acceptance criteria are concrete and verifiable, and the scope boundaries
  (out-of-scope, contracts that must not break) are correct.
- Nothing important is missing and nothing contradicts `AGENTS.md`.
- Every implementation leaf is bounded low/medium source complexity, not high/unresolved or sized by testing effort.
- The whole hierarchy covers the request without gaps/overlap, preserves real dependencies, and includes every
  required native parent link, category label, and `sub-issue` label.

Address the rubber-duck's findings in the affected drafts before moving on. Only proceed once the whole plan holds up
to review; scope changes after acceptance require this review again.

### 7. Present the reviewed draft for acceptance

**Always open every parent and child draft with mdview before asking the user to accept**, so they can review the
rendered issue:

```ps1
mdview tmp/triage-issue.md
```

Show a summary in chat including the proposed **title**, **categories/labels**, and any
**duplicate/related issues** surfaced in step 3 (with their `#number`s), then **ask the user to
accept, edit, or reject the whole plan**, including the hierarchy, reused issues, and source complexity, using the
`ask_user` tool (accept / edit / reject). No creation precedes explicit acceptance of all drafts. If they choose "edit",
let them edit `tmp/triage-issue.md` (and/or adjust categories) and wait for confirmation, then
re-read the file **and reopen it with `mdview tmp/triage-issue.md`** so they see the updated render.
Reopen every changed draft on subsequent modifications and obtain acceptance again; re-run step 6 for scope changes.

### 8. Create the GitHub issue — only if accepted

**Only run this step if the user accepted the whole plan in step 7.** If rejected, delete its drafts and stop.

Use the bundled helper script — it derives the repo root, ensures each label exists (creating any that are missing), extracts the title from the first `# ` heading, writes the body to a git-ignored temp file, creates the issue assigned to `@me`, and prints the title and URL:

```ps1
pwsh -NoProfile -ExecutionPolicy Bypass `
  -File .github/skills/triage/scripts/new-issue.ps1 `
  -Draft tmp/triage-issue.md -Label <cat1>,<cat2>
```

Pass the chosen categories as a single comma-separated `-Label` value (`pwsh -File` passes
arguments as literal strings, so repeating `-Label` fails to bind). Create accepted parents before their children,
using each issue's own draft and categories. For **every child**, pass `-Parent <parent_number>` to the same helper:
it ensures the `sub-issue` label and creates the native relationship via `gh issue create --parent`.
This also covers existing parents and recursively nested children.

For accepted **reused** children, ensure `sub-issue` exists using the helper's label list/create mechanism, then label
and attach the existing issue without creating a duplicate:

```ps1
gh issue edit <child_number> --add-label sub-issue
gh issue edit <parent_number> --add-sub-issue <child_number>
```

Link agreed prerequisites separately with `gh issue edit <dependent_number> --add-blocked-by <prerequisite_number>`.
Do not attach an issue merely because it is a prerequisite. On any creation/link failure, report partial results and
inspect GitHub before retrying; never blindly create the issue again.

**Link related issues.** The `Related to #NN` mentions in the body (from step 5)
already create GitHub cross-reference relationships on creation — no extra API call is needed. If a
related issue was discovered *after* the body was drafted, add the link with a cross-reference
comment instead:

```ps1
gh issue comment <new_issue_number> --body "Related to #NN — <reason>"
```

Then report the created issue URL to the user.

Verify each created/reused issue's labels with `gh issue view <number> --json labels`. For every planned child, confirm
the exact `sub-issue` label and expected native parent using `gh api repos/<owner>/<repo>/issues/<child_number>/parent`;
also check the parent's paginated `sub_issues` endpoint and each dependent's `dependencies/blocked_by` endpoint.
Body text or child counts do not prove relationships. Report any missing labels or links explicitly.

### 9. Cleanup

1. Delete the plan's draft files and any temp body file.
2. Show a short summary: all issue URLs, titles, assigned categories/labels, and verified parent/child links.
