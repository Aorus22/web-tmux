---
phase: "1"
status: findings
score: 7/10
created: 2026-09-06
---

# Phase 1 — UI Review (advisory)

**Audited:** 2026-09-06 (6-pillar visual/UX audit of implemented Rust GPUI code)
**Baseline:** `01-UI-SPEC.md` (approved, 2026-09-06) + FE parity anchors cited therein (`fe/src/components/layout/AppTitleBar.tsx`, `fe/src/App.tsx:213`, lucide-react 1.30.0)
**Screenshots:** Not captured — no dev server on :3000/:5173/:8080 and the build target is a native GPUI desktop app (no HTTP surface); this is a **code-only audit**
**Registry audit:** N/A — shadcn not initialized (UI-SPEC `shadcn_initialized: false`, Rust/GPUI target); no third-party UI registry blocks declared; gpui-component 0.6.0 crate source (cargo registry cache) inspected during the audit
**Contract additions audited against:** gpui-pre 0.3.3 semantics verified from `~/.cargo/registry/src` (token rem values, Animation default easing, `remove_window` behavior)

---

## Pillar Scores

| # | Pillar | Score | Status | Key Finding |
|---|--------|-------|--------|-------------|
| 1 | Design intents honored (tokens) | 8/10 | FLAG | Every SPEC color/size/spacing token is reproduced exactly (incl. derived `#9d9d9d`, `#c5c5c5`); 4 micro-deviations: separator gap 8px vs 4px, separator height 16px vs FE h-5, S2 missing `p_8`, unspec `shadow_lg` |
| 2 | Visual hierarchy & composition | 8/10 | PASS | S2 focal spinner, S3 700px failure card, S1 left-identity/right-controls all as specified; only off-contract element is the card shadow |
| 3 | Responsive/resize behavior | 6/10 | FLAG | Min-window clamp + restore guards solid; but S3 page has **no overflow-y scroll** (SPEC S3 explicitly requires it) and geometry is flushed only through the OS close path — the custom X button bypasses the save observer |
| 4 | Interaction affordances | 6/10 | FLAG | All 6 controls wired + cursor + hover bg; but hover **icon** tint does not apply (pinned child color), drag region covers only the center strip, max/restore icon desyncs on native maximize |
| 5 | States & feedback | 7/10 | FLAG | 2s-linear Loader2 + exact FE copy + tail-omission correct; stderr redaction masks **whole lines** (any substring hit of key/secret/token/password) instead of masking values per Assumption A4 |
| 6 | Consistency | 7/10 | FLAG | Icons consistent lucide stroke-1.5 (`Minus` path ≠ lucide 1.30.0); `theme.rs` palette helpers defined but views hardcode hex; gpui-component Theme bridge not fed the `default-dark` palette |

**Overall: 7/10** — `status: findings`. No blocker breaks the Phase 1 state machine, but two HIGH findings degrade core journeys (failed-page reachability on small windows; geometry persistence on the primary close path).

---

## Top Issues

| Severity | Issue | SPEC Ref | File:Line | Suggested Fix |
|----------|-------|----------|-----------|---------------|
| HIGH | **Custom Close button never persists geometry.** `window.remove_window()` only sets `self.removed = true` (gpui-pre `window.rs:2187-2188`) and never runs `on_window_should_close` observers, so the X button — the only visible close affordance on a frameless window — skips the settings flush. Position/size persist only when closed via OS-native paths (Alt+F4/taskbar), which frameless chrome de-emphasizes. | S4 ("persisted geometry … observe + close-flush"); Interaction Contract 5 | `tab_strip.rs:136-138` + `window_state.rs:177-187` | Give the close handler a shared flush: `extract_window_state(&window, prev)` → `save()` → `window.remove_window()`, mirroring the `observe()` body; or have `AppState` flush via `cx.on_release`-style teardown |
| HIGH | **Failed page (S3) has no overflow-y scroll.** `justify_center` + `size_full` with no `overflow_y_scroll`; with 10 wrapped stderr lines + heading + reason + buttons at min 456px body height, the card can exceed the viewport from both edges (centered overflow clips both ends) — Retry/Quit can be unreachable in the worst case. SPEC line 148: "Long content scrolls the page, not the card (overflow-y on the page area)." | S3, line 148; UI Considerations long-text row | `status.rs:79-87` | Add `.overflow_y_scroll()` on the S3 page container (or a scrollable wrapper with top-aligned content when card height > area) |
| HIGH | **Hover icon tint broken.** Parent sets `.hover(\|s\| s.text_color(#d4d4d4 / #ffffff))`, but each child `svg()` pins `.text_color(icon_idle_color #9d9d9d)` — explicit child style wins over inherited color, so icons stay idle-tint on hover. Close button hover becomes muted-gray-on-`#7F1D1D` instead of white-on-red per SPEC ("close hover: background `#7F1D1D` + icon `#ffffff`"). FE parity target: `hover:text-foreground` / `hover:text-destructive-foreground` (AppTitleBar.tsx:91,113,100). | S1, line 134; S1 hover contract | `tab_strip.rs:81-87, 102-111, 129-134` | Tag each button `.group("ctl")` and move the icon color onto the svg via `.group_hover("ctl", \|s\| s.text_color(...))`, or track hover with `on_mouse_enter`/`on_mouse_leave` state and pass the computed color into the `svg()` |
| MED | **Max/restore icon desyncs on native maximize.** Icon swaps on cached `app.is_maximized`, which is updated only by the button's own click listener. Double-click on the drag region / Aero-snap / Win+Up maximize the window (HTCAPTION behavior) without touching the flag → Square stays though maximized. SPEC: swap "by actual window state via `IsZoomed`". | S1, line 133; zero-one-many row (window controls) | `tab_strip.rs:105-109, 113-117`; `app_state.rs:36` | In the maximize branch call `crate::window_state::is_window_maximized(window)` (render has the `Window` via `AppState::render(&self, window, …)` — pass it down, replacing `_window`) instead of the cached flag |
| MED | **Stderr redaction over-masks.** Lines *containing* key/secret/token/password as a substring are replaced wholesale by `[REDACTED]` — e.g. a diagnostic "failed to parse token stream" is fully hidden. Assumption A4 specifies masking the **values** of matching lines, keeping the line. | S3, line 151; Assumptions A4 | `status.rs:22-29` | Mask the value segment only (split on `=`/`:`/whitespace after the keyword) and keep the rest of the line, e.g. `TOKEN_FALLBACK=[REDACTED]`; test with mixed lines |
| MED | **Geometry saved only on OS close-flush; SPEC asks observe + debounced save on move/resize/maximize.** Save-on-close alone loses state on crash/kill and (combined with the HIGH issue above) on the primary close path. | S4, line 160; 01-CONTEXT.md decisions | `window_state.rs:177-187`, `main.rs:86` | Extend `observe()` with window move/resize/maximize observers + a debounce (~300–500ms) writing `extract_window_state` into `DesktopSettings` |
| LOW | **Separator spacing off FE anchor.** Impl renders `ml_1` + `pl_1` + separator div with `mr_2` → **8px** separator→first-button gap; FE/SPEC spec `border-l` + `pl-1` = **4px**. Separator height 16px vs FE `h-5` (20px). | S1, line 132 (AppTitleBar.tsx:85) | `tab_strip.rs:61-69` | Drop `mr_2()` on the separator (4px gap comes from `pl_1` on the button row) and set separator height to `px(20.0)` |
| LOW | **Ready/Starting page paddings + unspec shadow.** S2 lacks the declared `p_8` 32px padding (visually inert — content is fixed-size and centered); failed card adds `shadow_lg()` which is not in the contract. | S2, line 141; S3, line 148 | `status.rs:43-51, 99` | Add `.p_8()` to the S2 root for contract fidelity; remove or SPEC-amend `shadow_lg()` |
| LOW | **Theme bridge not fed the FE palette; helpers dead.** `Theme::change(mode, None, cx)` leaves gpui-component on its *stock* dark palette, not the machine-ported `default-dark` values; Phase 1 looks correct only because views hardcode hex. Spec mandates machine-porting the palette "so later stock widgets follow." | Color, line 92 ("Machine-port into ThemePreset … wire the gpui-component Theme bridge") | `theme.rs:8-15, 18-23` | Build the web-term-derived palette struct (danger=danger_foreground mapping incl. `destructiveForeground→danger_foreground`) and pass it to `Theme::change`; route views through `theme.rs` helpers to kill duplicated hex literals |
| LOW | **No tooltips/labels on icon-only controls.** FE buttons carry native `title` + `aria-label` (Minimize/Maximize|Restore/Close); port renders bare SVGs. Tolerated by Assumption A7 (deferral), noted for Phase 3 parity, but the mx/restore tooltip must also know the actual state (same issue as the desync flag). | S1, line 137; Assumption A7 | `tab_strip.rs:72-139` | Phase 3: use the inventory `Tooltip` on the three controls; interim: wrap logic state in the toggle helper matching the live `IsZoomed` state |
| LOW | **Minimize icon ≠ lucide 1.30.0.** FE lucide-react 1.30.0 Minus is `M5 12h14` (verified `fe/node_modules/lucide-react/dist/esm/icons/minus.mjs`); ported SVG uses the older `<line x1="4" x2="20" y1="12" y2="12"/>` — ~4px wider stroke span breaks pixel parity. | Design System, line 29 | `icons.rs:5` | Replace the `line` node with `<path d="M5 12h14"/>` |

---

## Detailed Findings

### Pillar 1 — Design intents honored — 8/10 (FLAG)

Colors, spacing, sizes and typography are reproduced with high fidelity. Verified, not eyeballed:

- **Color tokens, exact hex-for-hex vs `default-dark`:** window/bar/body `#1e1e1e` (`tab_strip.rs:9,22`; `status.rs:49,85,120,180`; `app_state.rs:155`), card/hover `#2d2d2d` (`status.rs:94`; `tab_strip.rs:14,81,102`), foreground/accent `#d4d4d4` (`tab_strip.rs:13`; `status.rs:50,104,112,126,144,150,160`), borders `#3c3c3c` (`tab_strip.rs:10,24,68`; `status.rs:122,144`), destructive `#7F1D1D` (`status.rs:96,145`; `tab_strip.rs:129`), `#ffffff` on destructive (`tab_strip.rs:129`), muted `#808080` (`tab_strip.rs:11,37`; `status.rs:56,67`), derived idle `#9d9d9d` (`tab_strip.rs:12,86,111,134`), Retry hover blend `#c5c5c5` (`status.rs:161`). Accent discipline holds: `#d4d4d4` used as a **background** exactly once — Retry (sole primary CTA).
- **Geometry:** 44px bar ✓ (`tab_strip.rs:21`), `px_3` 12px ✓ (`:25`), `gap_2` 8px identity ✓ (`:32`), 40×32 controls ✓ (`:77-78,95-96,122-123`), radii `rounded_md`/`rounded_lg` = 6/8px ✓ (`:79,97,123,146,162`), icons 16/14/14/16 ✓ (`:36,110,133,85`), spinner 20px ✓ (`status.rs:55`), card `max_w(700px)`/`p_6`/radius 8 ✓ (`status.rs:93-98`), stderr inset `p_3`/6px radius ✓ (`status.rs:119,123`), buttons `px_4 py_2` ✓ (`status.rs:140-141,158-159`), footer `mt_6` + `gap_3` + `justify_end` ✓ (`status.rs:134-137`).
- **Typography vs scale:** `text_xs`=0.75rem=12px, `text_sm`=0.875rem=14px, `text_base`=1rem=16px confirmed from gpui-pre 0.3.3 `styled.rs:545-563`; used for Label 12 ("Starting backend…", `status.rs:65`), Body 14 (identity label, reason, buttons, `tab_strip.rs:41`; `status.rs:110,148,164`), Heading 16/500 ("Backend Startup Failed", `status.rs:102-103`). Weights: only `NORMAL`/400 and `MEDIUM`/500 appear — no `SEMIBOLD`/`BOLD` leakage from the web-term reference ✓. `text_xl`/Display 20 reserved, correctly unused.
- **Font family:** JetBrains Mono embedded + registered before first shape (`main.rs:43-44`). Caveat: only the stderr tail sets `.font_family("JetBrains Mono")` (`status.rs:125`); nothing pins the family globally, so all other text resolves to the text system's default family unless `.font_family` is inherited from a root element. SPEC Typography says "All Phase 1 text renders in it" — not provable from code. Fix: set `.font_family("JetBrains Mono")` on the root div in `app_state.rs::render` (it cascades).

**Deductions (−2):** separator→first-button gap 8px vs SPEC/FE 4px with height 16px vs FE 20px (`tab_strip.rs:64-69`); S2 missing `p_8` (`status.rs:43-51`); unspec `shadow_lg` (`status.rs:99`); global font family not guaranteed (see Pillar 6 note).

### Pillar 2 — Visual hierarchy & composition — 8/10 (PASS)

- S2: single focal point — 20px rotating spinner above a 12px muted label, centered, 12px gap — matches the FE screenshot contract (spinner + text only, no caption) ✓ (`status.rs:43-70`).
- S3: 700px card is the clear focal element over a bare body; hierarchy heading (16/500) → reason (14/400, `mt_2`) → mono inset tail (`mt_4`) → action row (`mt_6`, right-aligned, Quit outline before Retry primary) mirrors the web-term card structure the SPEC pinned ✓ (`status.rs:88-173`).
- S1: left identity cluster (16px muted TerminalSquare + 14px/500 "Tmux GUI"), right-aligned control triplet, 1px separator — composition matches the FE header ✓ (`tab_strip.rs:26-140`).
- Only off-contract compositional element: `shadow_lg()` on the failed card (`status.rs:99`) — an elevation cue the contract doesn't declare; visually harmless, but un-specified additions deserve either spec text or removal.

### Pillar 3 — Responsive/resize behavior — 6/10 (FLAG)

Good groundwork, two real gaps:

- ✅ `window_min_size 800×500` + default 1200×800 (`main.rs:52`; `window_state.rs:10-11`); restore path clamps `w∈[800,3840]`, `h∈[500,2160]`, rejects degenerate 0×0 and minimized `≤ -10000` coordinates, preserves windowed bounds under `Maximized` (`window_state.rs:144-174, 106-140`).
- ❌ **No overflow-y on the S3 page** (`status.rs:79-87`): SPEC line 148 explicitly requires "overflow-y on the page area". At the 800×500 minimum (456px body), heading + wrapped reason + up to 10 wrapped stderr lines + `mt_6` action row can exceed the viewport; with `justify_center` overflowing content clips at *both* edges and there is no scrollbar — the primary CTA can become unreachable. Severity HIGH (advisory).
- ⚠️ **Geometry persistence is close-flush only** (`window_state.rs:177-187`); SPEC S4 line 160 says "observe + debounced save on move/resize/maximize". Combined with the HIGH issue in the Top Issues table (custom X bypasses `on_window_should_close` entirely — verified against gpui-pre `window.rs:2187-2188`, which just sets `self.removed = true`), the primary close path likely saves nothing.
  - Note: quick-scale sanity is fine for S1 (fixed 44px chrome, static identity) per SPEC overflow/long-text rows — no workarounds needed there.

### Pillar 4 — Interaction affordances — 6/10 (FLAG)

- ✅ All controls wired: minimize → `window.minimize_window()` (`tab_strip.rs:88-90`), maximize/restore → `toggle_maximize` + `cx.notify()` (`:113-117`), close → `window.remove_window()` (`:136-138`), Retry → `start_supervisor` full re-handshake with status reset to Starting (`app_state.rs:162-165`, `:71-72`), Quit → `stop_supervisor()` then `cx.quit()` (`:166-168`); `cursor_pointer` on every clickable ✓; hover backgrounds ✓ (`#2d2d2d`, close `#7F1D1D`, Quit `#3c3c3c`, Retry `#c5c5c5`) ✓.
- ✅ Drag region exists via `WindowControlArea::Drag` (`tab_strip.rs:52`) — on Windows this is HTCAPTION, so native double-click-maximize and Aero snap come for free (the SPEC's double-click contract).
- ❌ **Drag coverage is partial**: FE makes the entire `<header>` a drag region with children opting out (`AppTitleBar.tsx:37-40`); the port only makes the center flex-1 strip draggable. Mousedown-drag on the left identity area and anywhere inside the right controls container (outside the buttons) is a dead zone.
- ❌ **Hover icon color does not apply** (see Top Issues #3): `svg().text_color(icon_idle_color)` pins the child color; the parent's `.hover(text_color)` cannot cascade into it. Background feedback works; the icon tint half of the hover contract (idle `#9d9d9d` → `#d4d4d4`; close → `#ffffff`) is dead code in practice.
- ⚠️ Max/restore icon driven by cached `app.is_maximized` (seeded from settings at boot, updated only on button click) instead of live `is_window_maximized` — native maximize paths desync the glyphs. SPEC line 133 requires the swap to track **actual window state**.
- ⚠️ No tooltips on icon-only buttons — SPEC A7 sanctions deferral to Phase 3, so this is tolerated, but record it: FE had `title` + `aria-label` on all three controls.

### Pillar 5 — States & feedback — 7/10 (FLAG)

- ✅ **Loading state S2 is exact**: `Loader2` path byte-matches FE lucide (`loader-circle.mjs`: `M21 12a9 9 0 1 1-6.219-8.56` — lucide-react 1.30.0), 20px, `#808080`, and `Animation::new(2s).repeat()` — gpui-pre default easing is **linear** (`elements/animation.rs:32-38`), matching the 2s-linear-infinite contract without an explicit `.with_easing`. Copy is exact incl. U+2026 (`status.rs:68` ↔ `App.tsx:213`).
- ✅ **Timeout integrity**: 10s handshake/readiness timeouts live in the supervisor, so a stalled spawn lands on S3, not a perpetual spinner (supervisor `lib.rs:64` + plan-verified tests).
- ✅ **Empty/overflow tail handling**: zero-line tail omits the block entirely (`status.rs:75,115`); supervisor ring keeps newest 10; per-line divs wrap; empty states per SPEC.
- ❌ **Redaction over-applies** (`status.rs:22-29`): whole-line `[REDACTED]` on any substring hit of key/secret/token/password (case-insensitive). A4 specifies masking *values*; wholesale masking deletes diagnostic signal (e.g. an error line mentioning "token" fades to `[REDACTED]`), which is exactly the failed page's job to communicate. Also note the supervisor ring stores raw stderr (no redaction there) — view-side redaction covers the rendered surface, but A4's defense-in-depth wording is only satisfied at one layer.
- ✅ Retry path resets to Starting and re-renders S3 fresh on repeated failure (`app_state.rs:71,129-134`, watch→mpsc pump with `weak.upgrade()`); no confirmation dialogs per contract; Quit kills a live sidecar first.
- ✅ S4 renders S1 over a bare `#1e1e1e` body with zero status-page leftovers (`status.rs:176-182`).
- Feedback gap noted under Pillar 4 (hover icons) is not double-counted here.

### Pillar 6 — Consistency — 7/10 (FLAG)

- ✅ Icon discipline: all six Phase 1 icons are embedded lucide constants at `stroke-width="1.5"` exactly per FE `stroke-[1.5]`; no gpui-component `IconName` glyphs on chrome as mandated. Code verified against lucide-react 1.30.0: Loader2/loader-circle path exact, Square/Copy/X/TerminalSquare match current lucide geometry; **Minus** is the one outlier (`line 4..20` vs lucide `M5 12h14`, verified in fe/node_modules) — fix in Top Issues.
- ❌ **Theme single-source-of-truth is broken rather than violated:** `theme.rs` declares the exact SPEС palette as helpers (`bg_color`, `card_bg`, `fg_color`, `muted_fg`, `border_color`, `destructive_color`, `theme.rs:18-23`) — and **no view uses them**; `tab_strip.rs:9-14` and `status.rs` re-hardcode the same hex values. Values are correct but triplicated; a Phase 6 palette change would require hunting literals.
- ❌ **gpui-component Theme bridge is not fed the `default-dark` palette** (`theme.rs:14`, `Theme::change(mode, None, cx)`): Phase 1 surfaces are unaffected because they hardcode tokens, but the SPEC's Color section (line 92) requires the machine-ported palette "so later stock widgets follow" —.Phases that consume stock `Button`/`Dialog` will inherit gpui-component's stock dark palette and repaint off-contract.
- ⚠️ `apply_theme(System → Dark)` silently substitutes the System variant (documented Phase 6 placeholder; acceptable this phase, worth a TODO marker).
- Module hygiene is fine otherwise: `views/mod.rs` minimal, generic `render_status_page<V>` keeps status presentation reusable across hosts.

---

## Files Audited

- `.planning/phases/01-workspace-foundation-backend-sidecar/01-UI-SPEC.md` (contract of record)
- `.planning/phases/01-workspace-foundation-backend-sidecar/01-CONTEXT.md`
- `.planning/phases/01-workspace-foundation-backend-sidecar/01-0{1,2,3}-PLAN.md` + `-SUMMARY.md`
- `desktop-gpui/crates/webtmux/src/views/tab_strip.rs` (S1)
- `desktop-gpui/crates/webtmux/src/views/status.rs` (S2/S3/S4 + redaction)
- `desktop-gpui/crates/webtmux/src/views/mod.rs`
- `desktop-gpui/crates/webtmux/src/theme.rs`
- `desktop-gpui/crates/webtmux/src/window_state.rs`
- `desktop-gpui/crates/webtmux/src/app_state.rs`
- `desktop-gpui/crates/webtmux/src/main.rs`
- `desktop-gpui/crates/webtmux/src/icons.rs`
- Parity anchors: `fe/src/components/layout/AppTitleBar.tsx`, `fe/src/App.tsx` (line 213), `fe/src/features/sessions/ErrorState.tsx`, lucide-react 1.30.0 (`minus.mjs`, `loader-circle.mjs`)
- Reference sources: `desktop-gpui/crates/supervisor/src/lib.rs` (BackendStatus/ring/10s timeouts), gpui-pre 0.3.3 (`styled.rs` token scale, `elements/animation.rs` easing, `window.rs` remove/close semantics)

*Advisory review — no finding above is blocking; source code was not modified.*
