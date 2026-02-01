# Plan: Sidebar Filters & CountBadge Component

Implementação de filtros inteligentes na barra lateral e criação do componente `CountBadge` para exibição de estatísticas com excelência UX.

## 1. Backend: Optimized Statistical Query
- Implementar uma query SQL única no Rust ou expor via IPC para buscar contagens de:
  - Total de imagens (All Items).
  - Imagens sem tags (Untagged).
  - Contagem por Folder (`locations`).
  - Contagem por Tag (incluindo hierarquia se necessário).

## 2. Component: `CountBadge`
- Local: `src/components/ui/CountBadge.tsx`
- Funcionalidades:
  - Abreviação de números (ex: 1200 -> 1.2k).
  - Tooltip customizado via CSS mostrando o valor real.
  - Design acessível e alinhado ao sistema.

## 3. Library Sidebar Overhaul
- **All Items**: Reset de filtros e exibição do total global.
- **Untagged**: Filtro específico para `images.id NOT IN (SELECT image_id FROM image_tags)`.
- **Folders**: Exibir contagem por pasta.
- **Tags**: Exibir contagem por tag no `TreeView`.
- **UX**: Feedback visual "active" elegante (fundo/destaque) para o item selecionado.

## 4. Store Integration
- Atualizar o `AppState` para suportar filtros de Folder e Untagged.
- Criar ações para lidar com a seleção exclusiva (All Items) vs combinada.

## Verification Criteria
- [x] `CountBadge` abrevia números e mostra tooltip.
- [x] Sidebar mostra contagens em tempo real.
- [x] Filtro "Untagged" funciona corretamente.
- [x] Feedback visual mostra claramente o filtro ativo.
- [x] Performance de consulta permanece fluida.
