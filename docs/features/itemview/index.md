# ItemView

Full-screen overlay used to preview an asset.

## Header

`ItemViewHeader` (`src/components/features/itemview/common/ItemViewHeader.tsx`) is rendered above the toolbar. Because the overlay covers the application `TitleBar`, the header:

- Shows the file name of the asset being viewed (ellipsis truncation, full name as native tooltip).
- Follows the `TitleBar` look (height, border, macOS traffic-light inset).
- Is draggable via `data-tauri-drag-region`, so the window can be moved while the viewer is open.
- Renders `WindowControls` (Windows/Linux only) so the window can still be minimized, maximized or closed.
