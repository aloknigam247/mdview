# Rust crates

Use the [root crate index](../AGENTS.md#crate-index) to select a package, then its `Cargo.toml` and `src/lib.rs`.

## Plugin contract

Do not break `MdViewExtension` in [mdview-core/src/ext.rs](mdview-core/src/ext.rs).
Its source defines the parser, transformation, per-node rendering, document pre-rendering, and asset hooks.

| Core file | Read for |
|-----------|----------|
| [parser.rs](mdview-core/src/parser.rs) | GFM setup and parse-hook lifecycle |
| [registry.rs](mdview-core/src/registry.rs) | Registration order and hook dispatch |
| [types.rs](mdview-core/src/types.rs) | HTML, terminal chunks, themes, and render context |

## Extension points

To add an extension:

1. Create `mdview-ext-<name>` with explicit path dependencies on the shared crates it uses.
2. Implement the contract; use [mdview-ext-math/src/lib.rs](mdview-ext-math/src/lib.rs) for selective node matching
   or [mdview-ext-frontmatter/src/lib.rs](mdview-ext-frontmatter/src/lib.rs) for document pre-render hooks.
3. Add the app dependency and register in [apps/mdview/src/builtins.rs](../apps/mdview/src/builtins.rs).
   Do not register in the [unused core stub](mdview-core/src/builtins.rs).
   Registration order is semantic: selective fenced-block handlers must precede the catch-all `Highlight`, which
   must remain last. Do not alphabetize this registry.
4. Wire client requirements through the actual [app page path](../apps/mdview/AGENTS.md#native-host-and-page);
   declaring `client_assets` alone does not load a library into that page.
5. Add focused in-crate tests and a fixture under [fixtures/](../fixtures/); use the
   [surface validation policy](../.github/triage.md#validation) for acceptance.

For a new output surface, add `mdview-render-<name>` around the AST/render context and wire mode selection and dispatch
through the [app entrypoints](../apps/mdview/AGENTS.md#entrypoints).

## Renderers

- App rendering and library rendering are separate paths; start with the
  [app map](../apps/mdview/AGENTS.md#entrypoints) for user-visible changes.
- HTML library behavior starts at [mdview-render-html/src/lib.rs](mdview-render-html/src/lib.rs); its `_stubs.rs`
  defines a separate registry/context alongside core re-exports. Do not interchange identically named types.
- Server-library behavior starts at [mdview-server/src/lib.rs](mdview-server/src/lib.rs); its Axum/WebSocket API
  is separate from the app's `server.rs`.
- Terminal-library behavior starts at [mdview-render-terminal/src/lib.rs](mdview-render-terminal/src/lib.rs).
  Pager interaction starts at [mdview-pager/src/lib.rs](mdview-pager/src/lib.rs).

## Dependencies and validation

- Each crate must pass `cargo test --locked --package <crate>` in isolation, not only as part of the workspace.
- Extensions must not depend on each other. Import real sibling types through declared dependencies rather than
  duplicating them behind local stubs.
- Reuse the libraries already declared in package manifests instead of reimplementing their parsing, rendering,
  highlighting, or transport facilities.
- Scoped guides cover [configuration](mdview-config/AGENTS.md), [highlighting](mdview-ext-highlight/AGENTS.md),
  and [themes](mdview-theme/AGENTS.md). Rust integration tests live in each package's `tests/`;
  examples and snapshot entrypoints are declared in its manifest.
