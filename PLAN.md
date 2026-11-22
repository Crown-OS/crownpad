# crownpad rewrite — LaunchOS feature parity on crownui

Stack: crownui (wayland + impeller + tokio), crownconfig for every persisted byte,
crownos-ipc (tokio adapter) for show/hide/toggle. Backdrop and trays use crownui's
`Material::thick()` / `Material::thin()` tinted from `appearance.ron` (no liquid glass).

## Persisted state (`~/.config/crownos/*.ron`, via crownconfig)
- `crownpad.ron` — settings: grid rows/columns, browsing mode, labels, sort mode,
  reopen position, click-empty-to-close, hidden-in-search, extra app sources.
- `crownpad-layout.ron` — pages of items, folders, hidden apps, aliases, launch stats.
- `dock.ron` — written only to pin an app (Add to Dock).

## Steps
1. [x] Foundation: crownui manifest, module tree, settings + layout sections,
       desktop-entry catalog, icon decode, pure library model (pages, folders), tests.
2. [x] Shell: fullscreen overlay layer, show/hide spring driving the thick material,
       keyboard interactivity + input region follow visibility, IPC server + crownpadctl,
       Esc / click-empty closes, launch detached through `setsid`.
3. [x] Grid: paged mode with page dots, wheel / swipe / drag paging, Ctrl+←/→,
       vertical scroll mode, auto icon size from rows × columns and surface size.
4. [x] Search: type-anywhere, ranked fuzzy match (exact, prefix, word prefix, initials,
       camel case, substring, subsequence) over name + alias, arrow navigation, Enter opens.
5. [x] Folders: drop app on app to create, tray on `Material::thin()`, rename title,
       dissolve, auto-dissolve at one app, scrolling folder contents, drag reorder inside.
6. [x] Drag & drop: long-press/drag reorder with live preview, edge page flip, new page,
       system DnD (`text/uri-list`) so icons drop onto crowndock, multi-select
       (Ctrl-click, rubber band) with batch move / folder / hide / open.
7. [x] Context menus: app (open, show in files, add to dock, move to folder, move to page,
       rename, hide, uninstall), folder (rename, dissolve, move to page),
       empty area (sort mode, browsing mode, reorganize, settings).
8. [x] Sorting: manual / name / date added / last opened / frequency; one-shot reorganize
       by name, install time, last opened, icon colour; ungroup all; folders first/last; reset.
9. [~] App management: catalog rescans on every open (no inotify yet); `app_sources` is config-only,
       hidden-app restore, uninstall (pacman / flatpak via pkexec) with confirmation.
10. [x] Settings sheet inside the pad (kit widgets), live-follow of every config edit,
        reopen position (last / first / first after delay), label toggles.
11. [~] Polish: clippy -D warnings and 61 tests pass; VRAM of the always-mapped fullscreen surface not yet profiled.

## Out of scope (macOS-only or compositor-owned)
F4 key, pinch / hot-corner triggers, Dock-menu app list, display selection
(the compositor places the overlay), Full Disk Access cleanup of leftovers.

## Verified live (CrownOS session, ydotool + grim)
show/hide/toggle over IPC, blurred thick backdrop, paged grid with real icons, page dots,
type-anywhere search with top-result highlight, Esc levels, Ctrl+→ paging, arrow highlight.

## Not yet exercised live
pointer drag reorder, folder create/tray/rename, drop onto crowndock, rubber band,
context menus, settings sheet, uninstall dialog.
