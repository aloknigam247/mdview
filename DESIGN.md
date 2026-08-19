# mdview design guideline

## Purpose and scope

`DESIGN.md` is mdview's source of truth for the GUI visual language. It governs
`apps/mdview/src/render.rs::wrap_page` first, because that wry webview is the main GUI surface and
currently owns the inline CSS for page layout, table styling, code blocks, menus, banners, the TOC,
and the codemap.

The standalone HTML renderer should mirror the shared heading, typography, table, and code-block
rules where `.mdv-doc` maps to the same rendered concepts. Terminal and Neovim surfaces are in scope
only when they consume shared theme tokens; their layout and interaction models remain separate.

This document is inspiration-only: mdview does not adopt Fluent components or Fluent implementation
packages. A later implementation issue depends on this guideline and may translate these documented
tokens into Rust, CSS, and theme code. This document itself changes no runtime behavior.

## Sources

The guideline derives its metrics, placement language, and color roles from these sources:

- Fluent 2 design tokens: <https://fluent2.microsoft.design/design-tokens>
- Fluent 2 elevation: <https://fluent2.microsoft.design/elevation>
- Fluent 2 motion: <https://fluent2.microsoft.design/motion>
- Fluent 2 typography: <https://fluent2.microsoft.design/typography>
- Fluent UI web token package values:
  <https://unpkg.com/@fluentui/tokens@1.0.0-alpha.24/lib/global/index.js>
- Catppuccin style guide:
  <https://github.com/catppuccin/catppuccin/blob/main/docs/style-guide.md>
- Catppuccin palette: <https://catppuccin.com/palette/>
- mdview shipped presets:
  `crates/mdview-theme/src/themes/catppuccin_latte.rs` and
  `crates/mdview-theme/src/themes/catppuccin_mocha.rs`

Fluent 2 provides the named radius, elevation, motion, typography, spacing, and placement concepts.
Catppuccin provides the color role system and exact hex values. The shipped presets are the
repository-local consistency check for aliases that already exist in mdview.

## Color system: Catppuccin roles

Use Catppuccin roles semantically rather than by perceived brightness alone. Body backgrounds use
`base`; secondary panes use `mantle` or `crust`; raised surfaces use `surface0` through `surface2`;
subtle strokes, disabled marks, and selections use `overlay0` through `overlay2`; copy uses `text`
and `subtext0` through `subtext1`. Accents are sparse and purposeful: links use blue, success uses
green, warnings use yellow, errors use red, and decorative emphasis may use mauve, teal, or peach.

Legibility always comes first. If an accent background would make Catppuccin's normal text role fail
contrast, use the role that gives readable text on that background instead of preserving a purely
mechanical role mapping.

| Role | Intended usage | Latte | Mocha | mdview alias status |
| --- | --- | --- | --- | --- |
| `base` | Primary page background and default document canvas | `#eff1f5` | `#1e1e2e` | Existing `bg` |
| `mantle` | Secondary panes behind floating surfaces | `#e6e9ef` | `#181825` | New role |
| `crust` | Strong separators, outer app edges, low-emphasis borders | `#dce0e8` | `#11111b` | Existing `border_subtle`, `table_border` |
| `surface0` | Code blocks, cards, and quiet embedded surfaces | `#ccd0da` | `#313244` | Existing `code_bg` |
| `surface1` | Hovered rows, selected code lines, and raised surface fills | `#bcc0cc` | `#45475a` | Existing `code_hl_bg` |
| `surface2` | Active controls, stronger chips, and pressed surface fills | `#acb0be` | `#585b70` | New role |
| `overlay0` | Disabled strokes, minimap tracks, low-emphasis dividers | `#9ca0b0` | `#6c7086` | New role |
| `overlay1` | Subtle icons, secondary dividers, and quiet metadata | `#8c8fa1` | `#7f849c` | New role |
| `overlay2` | Selection background at 20-30% opacity and focus-adjacent marks | `#7c7f93` | `#9399b2` | New role |
| `subtext0` | Muted body text, captions, and helper copy | `#6c6f85` | `#a6adc8` | Existing `muted` |
| `subtext1` | Block quotes and secondary headings | `#5c5f77` | `#bac2de` | Existing `quote_fg` |
| `text` | Body copy, primary headings, and code foreground | `#4c4f69` | `#cdd6f4` | Existing `fg` |
| `blue` | Links, primary accent, focused actionable affordances | `#1e66f5` | `#89b4fa` | Existing `accent`, `accent_blue`, `link` |
| `green` | Success and affirmative status | `#40a02b` | `#a6e3a1` | Existing `accent_green` |
| `mauve` | Secondary heading accent and special emphasis | `#8839ef` | `#cba6f7` | Existing `accent_mauve` |
| `peach` | Warm decorative heading accent and non-error attention | `#fe640b` | `#fab387` | Existing `accent_peach` |
| `red` | Errors, destructive markers, and inline-code foreground | `#d20f39` | `#f38ba8` | Existing `code_inline_fg` |
| `teal` | Informational accent and calm decorative emphasis | `#179299` | `#94e2d5` | Existing `accent_teal` |
| `yellow` | Warnings and config-error banner emphasis | `#df8e1d` | `#f9e2af` | Existing `accent_yellow` |

Existing aliases must keep their current values until the implementation issue deliberately migrates
them. New roles may be added without changing the meaning of `Theme::colors` entries that already
exist.

## Shape and corner radius scale

Use Fluent's global radius tokens as the target shape scale. mdview should remain modern, sleek, and
curved: default document primitives are softly rounded, while menus and panels get stronger radius.

| Token | Value | mdview usage |
| --- | --- | --- |
| `borderRadiusNone` | `0` | Full-bleed seams and table cell joins where rounding would misalign |
| `borderRadiusSmall` | `2px` | Tiny affordances, focus insets, and compact pills |
| `borderRadiusMedium` | `4px` | Inline code, copy buttons, and small controls |
| `borderRadiusLarge` | `6px` | Table row highlight masks and compact floating controls |
| `borderRadiusXLarge` | `8px` | Code-block interiors and grouped controls |
| `borderRadius2XLarge` | `12px` | Tables, cards, TOC, menus, banners, and help panels |
| `borderRadius3XLarge` | `16px` | Large panels and page-level cards |
| `borderRadius4XLarge` | `24px` | Hero or modal containers if introduced later |
| `borderRadius5XLarge` | `32px` | Reserved for oversized decorative surfaces |
| `borderRadius6XLarge` | `40px` | Reserved for oversized decorative surfaces |
| `borderRadiusCircular` | `10000px` | Pills, round buttons, and circular indicators |

The current theme contract exposes `Radii.sm/md/lg = 4/10/16`. Treat those as compatibility aliases
until implementation expands the token set: `sm` maps closest to `borderRadiusMedium`,
`md` maps to a local bridge between `borderRadiusXLarge` and `borderRadius2XLarge`, and
`lg` maps to `borderRadius3XLarge`.
`borderRadius3XLarge`.

## Elevation ramp

surfaces and avoid shadows on strongly colored accent surfaces unless the implementation also
applies Fluent's luminosity-adjusted brand shadow model.
Fluent's luminosity-adjusted brand shadow model.

| Token | Light value | Dark value | mdview usage |
| --- | --- | --- | --- |
| `shadow2` | `0 0 2px rgba(0,0,0,0.12), 0 1px 2px rgba(0,0,0,0.14)` | `0 0 2px rgba(0,0,0,0.24), 0 1px 2px rgba(0,0,0,0.28)` | Pressed floating buttons and low cards |
| `shadow4` | `0 0 2px rgba(0,0,0,0.12), 0 2px 4px rgba(0,0,0,0.14)` | `0 0 2px rgba(0,0,0,0.24), 0 2px 4px rgba(0,0,0,0.28)` | Default raised cards and compact panels |
| `shadow8` | `0 0 2px rgba(0,0,0,0.12), 0 4px 8px rgba(0,0,0,0.14)` | `0 0 2px rgba(0,0,0,0.24), 0 4px 8px rgba(0,0,0,0.28)` | TOC, copy controls, tooltips, and command surfaces |
| `shadow16` | `0 0 2px rgba(0,0,0,0.12), 0 8px 16px rgba(0,0,0,0.14)` | `0 0 2px rgba(0,0,0,0.24), 0 8px 16px rgba(0,0,0,0.28)` | Hover cards, context menus, and warning banners |
| `shadow28` | `0 0 8px rgba(0,0,0,0.12), 0 14px 28px rgba(0,0,0,0.14)` | `0 0 8px rgba(0,0,0,0.24), 0 14px 28px rgba(0,0,0,0.28)` | Side panels and high-priority overlays |
| `shadow64` | `0 0 8px rgba(0,0,0,0.12), 0 32px 64px rgba(0,0,0,0.14)` | `0 0 8px rgba(0,0,0,0.24), 0 32px 64px rgba(0,0,0,0.28)` | Modal-scale panels if introduced later |

Avoid using elevation as the only state indicator. Pair raised surfaces with borders from `crust`,
`overlay0`, or `surface1` so dark and light themes both keep visible structure.

## Motion

Use Fluent duration and easing tokens for GUI transitions. All motion must honor
`prefers-reduced-motion: reduce` by switching transforms and long fades to zero-duration state
changes while preserving focus and layout changes.

| Token | Value | mdview usage |
| --- | --- | --- |
| `durationUltraFast` | `50ms` | Tap highlights and tiny opacity changes |
| `durationFaster` | `100ms` | Button hover and copy-feedback transitions |
| `durationFast` | `150ms` | Menu item hover, TOC item hover, and row emphasis |
| `durationNormal` | `200ms` | Default fades for panels and banners |
| `durationGentle` | `250ms` | Slightly larger panel entrance or exit |
| `durationSlow` | `300ms` | Codemap or TOC reposition transitions |
| `durationSlower` | `400ms` | Reserved for rare large layout transitions |
| `durationUltraSlow` | `500ms` | Avoid in normal interactions; reserved for demos |
| `curveAccelerateMax` | `cubic-bezier(0.9,0.1,1,0.2)` | Fast exit when an element leaves focus |
| `curveAccelerateMid` | `cubic-bezier(1,0,1,1)` | Medium exit |
| `curveAccelerateMin` | `cubic-bezier(0.8,0,0.78,1)` | Gentle exit |
| `curveDecelerateMax` | `cubic-bezier(0.1,0.9,0.2,1)` | Prominent entrance |
| `curveDecelerateMid` | `cubic-bezier(0,0,0,1)` | Default entrance |
| `curveDecelerateMin` | `cubic-bezier(0.33,0,0.1,1)` | Subtle entrance |
| `curveEasyEaseMax` | `cubic-bezier(0.8,0,0.2,1)` | Container transform or resize |
| `curveEasyEase` | `cubic-bezier(0.33,0,0.67,1)` | Default hover and fade easing |
| `curveLinear` | `cubic-bezier(0,0,1,1)` | Progress-only motion; avoid for UI entrances |

Use quick, natural durations. Avoid flashes, unrelated page-wide movement, and animation that
continues outside the element currently changing state.

## Typography ramp

Use Fluent's web type ramp and font stacks for GUI typography. The default body stack is:
`'Segoe UI', 'Segoe UI Web (West European)', -apple-system, BlinkMacSystemFont, Roboto,
'Helvetica Neue', sans-serif`.

The heading stack is the same as the body stack so headings inherit platform-native rendering. The
mono stack is: `Consolas, 'Courier New', Courier, monospace`. Numeric UI may use:
`Bahnschrift, 'Segoe UI', 'Segoe UI Web (West European)', -apple-system, BlinkMacSystemFont, Roboto,
'Helvetica Neue', sans-serif`.

| Token | Weight | Size / line-height | mdview usage |
| --- | --- | --- | --- |
| `caption-2` | Regular 400 | `10px / 14px` | Dense helper text if introduced later |
| `caption-1` | Regular 400 | `12px / 16px` | TOC metadata, minimap labels, and shortcuts |
| `body-1` | Regular 400 | `14px / 20px` | UI chrome, menus, buttons, and compact panels |
| `ui-body` | Regular 400 | `16px / 22px` | Reading surface minimum for dense markdown |
| `subtitle-2` | Semibold 600 | `16px / 22px` | Small panel titles and table headers |
| `subtitle-1` | Semibold 600 | `20px / 28px` | Section headings and prominent panel titles |
| `title-3` | Semibold 600 | `24px / 32px` | Markdown `h3` when visual hierarchy needs it |
| `title-2` | Semibold 600 | `28px / 36px` | Markdown `h2` |
| `title-1` | Semibold 600 | `32px / 40px` | Markdown `h1` |
| `large-title` | Semibold 600 | `40px / 52px` | Optional hero title; not default markdown |
| `display` | Semibold 600 | `68px / 92px` | Out of scope for normal documents |

Keep long-form markdown left-aligned for left-to-right content. Use color to reinforce hierarchy,
not to replace it: standard text needs at least 4.5:1 contrast, and large text needs at least 3:1.

## Spacing scale

Use Fluent horizontal and vertical spacing tokens. Horizontal and vertical names share the same raw
values; choose the axis-specific token in implementation so code remains self-documenting.

| Token suffix | Horizontal token | Vertical token | Value | mdview usage |
| --- | --- | --- | --- | --- |
| `None` | `spacingHorizontalNone` | `spacingVerticalNone` | `0` | Joined cells and reset edges |
| `XXS` | `spacingHorizontalXXS` | `spacingVerticalXXS` | `2px` | Hairline offsets and focus insets |
| `XS` | `spacingHorizontalXS` | `spacingVerticalXS` | `4px` | Inline code padding and icon gaps |
| `SNudge` | `spacingHorizontalSNudge` | `spacingVerticalSNudge` | `6px` | Compact control padding |
| `S` | `spacingHorizontalS` | `spacingVerticalS` | `8px` | Default small gaps and row padding |
| `MNudge` | `spacingHorizontalMNudge` | `spacingVerticalMNudge` | `10px` | Slightly denser panel padding |
| `M` | `spacingHorizontalM` | `spacingVerticalM` | `12px` | Default control and table-cell padding |
| `L` | `spacingHorizontalL` | `spacingVerticalL` | `16px` | Card padding and section gaps |
| `XL` | `spacingHorizontalXL` | `spacingVerticalXL` | `20px` | Panel padding and document gutters |
| `XXL` | `spacingHorizontalXXL` | `spacingVerticalXXL` | `24px` | Major section rhythm |
| `XXXL` | `spacingHorizontalXXXL` | `spacingVerticalXXXL` | `32px` | Page gutters and large block separation |

Prefer spacing tokens over ad hoc pixels. If a component needs a value between tokens, first try the
nudge token rather than inventing a local number.

## Component placement and token mapping

Every GUI component should map visual styling and placement to named tokens. Use `N/A` where a
component deliberately has no border, radius, elevation, spacing, or fixed placement.

| Component | Background | Border | Radius | Elevation | Spacing | Placement |
| --- | --- | --- | --- | --- | --- | --- |
| Page body | `base` | `N/A` | `N/A` | `N/A` | `spacingHorizontalXXXL` gutters, `spacingVerticalXXL` block rhythm | Static document flow; max content width should remain readable, centered when viewport exceeds the chosen measure; layer `0` |
| Headings `h1`-`h3` | `N/A` | Optional underline `crust` for `h1` only | `N/A` | `N/A` | `spacingVerticalXXL` before, `spacingVerticalM` after | Static document flow; anchor links align to heading baseline; no fixed layer |
| `article.mdv pre` | `surface0` | `1px crust` | `borderRadiusXLarge` outer, `borderRadiusMedium` inner controls | `shadow2` only when raised from page | `spacingHorizontalL`, `spacingVerticalM`; code uses configured tab width | Static block with horizontal overflow inside the block, not on the page; copy button anchors top-right within the block; layer `1` |
| `article.mdv table` / `th` | Table `surface0`, header `surface1` | `1px crust`; row separators `overlay0` | `borderRadius2XLarge` clipped around table wrapper | `N/A` by default | Cells `spacingHorizontalM` and `spacingVerticalS` | Static block; horizontal overflow stays in wrapper; header remains in normal flow unless sticky headers are added later; layer `1` |
| Inline code and `.mdv-copy` | Inline code `surface0`; copy button `surface1` with hover `surface2` | `1px crust` for buttons, `N/A` for inline code | Inline code `borderRadiusMedium`; copy button `borderRadiusCircular` | Copy button `shadow8` when floating | Inline code `spacingHorizontalXS` / `spacingVerticalXXS`; copy button `spacingHorizontalS` | Inline code flows with text; `.mdv-copy` anchors top-right inside code blocks with `spacingHorizontalS` offset; layer `3` |
| `.mdv-toc` | `mantle` or `surface0` | `1px crust` | `borderRadius2XLarge` | `shadow8` | `spacingHorizontalL`, `spacingVerticalL` | Fixed or floating per config: right/left/center anchor, `spacingHorizontalXL` viewport offset, max-height below viewport chrome, responsive collapse below narrow widths, layer `20` |
| `#mdv-minimap` | Track `mantle`, marks `overlay0`/`overlay1`, viewport `blue` at low opacity | `1px crust` | `borderRadiusCircular` for track | `shadow4` if detached from edge | `spacingHorizontalXS` internal marks | Fixed right edge, vertically centered or full document rail, width compact enough to avoid content overlap, hidden on narrow viewports, layer `15` |
| `#mdv-context-menu` | `mantle` | `1px crust` | `borderRadius2XLarge` | `shadow16` | `spacingHorizontalS`, `spacingVerticalS` around items | Fixed at pointer position, clamped to viewport with `spacingHorizontalS` margin, above document chrome, layer `40` |
| `.mdv-help-panel` | `mantle` | `1px crust` | `borderRadius3XLarge` | `shadow28` | `spacingHorizontalXL`, `spacingVerticalXL` | Fixed centered panel with max-width and max-height, scrolls internally, dismissible with Escape, layer `50` |
| `#mdv-config-banner` | Warning uses `yellow` at low opacity over `mantle` | `1px yellow` or `1px crust` fallback | `borderRadius2XLarge` | `shadow16` | `spacingHorizontalL`, `spacingVerticalM` | Fixed top center below window chrome, width clamped to viewport minus `spacingHorizontalXL`, close button anchors right, layer `60` |

The `.mdv-doc` standalone HTML surface mirrors the heading, typography, code-block, and table rows
from this table. It does not need TOC, minimap, context-menu, help-panel, or config-banner mappings
unless those components are later added to standalone HTML output.

## Non-goals

This guideline does not adopt Fluent component libraries, replace mdview's Rust renderer, or require
web dependencies. Fluent is the metric and placement reference, not a component implementation.

This guideline does not change runtime behavior, visual CSS, Rust code, configuration, markdown
parsing, TOC behavior, codemap behavior, keymaps, context menus, live reload, terminal output, or
Neovim preview behavior.

This guideline does not remove non-Catppuccin presets. Catppuccin is the source for this design
language, while any theme-set refactor is tracked separately. Creating `.github/agents/design.md` is
also out of scope for this documentation issue.
