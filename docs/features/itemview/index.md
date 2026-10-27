# ItemView

Full-screen overlay used to preview an asset.

## Header

`ItemViewHeader` (`src/components/features/itemview/common/ItemViewHeader.tsx`) is rendered above the toolbar. Because the overlay covers the application `TitleBar`, the header:

- Shows the file name of the asset being viewed (ellipsis truncation, full name as native tooltip).
- Follows the `TitleBar` look (height, border, macOS traffic-light inset).
- Is draggable via `data-tauri-drag-region`, so the window can be moved while the viewer is open.
- Renders `WindowControls` (Windows/Linux only) so the window can still be minimized, maximized or closed.

## Ferramentas de imagem

- **Seleção** (`V`, padrão): cliques têm comportamento normal e a imagem pode ser arrastada para outros programas (reutiliza a diretiva `assetDragSource` da galeria).
- **Pan** (`H`) e **Rotação** (`R`): com `Shift` pressionado, a rotação trava em múltiplos de 45°.
- **Reset View** (`Mod+2` ou botão ao lado de Fit/Original): remove rotação e espelhamento e reajusta pan e zoom ao estado inicial (fit), mantendo a ferramenta ativa.
