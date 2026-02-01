# Task: Refatoração da Interface e Implementação do Viewport

Este plano descreve a reorganização da interface do Elleven-Library, introduzindo redimensionamento dinâmico em todos os contextos e a criação de um sistema de Viewport robusto para navegação e visualização de arquivos.

## 🎯 Objetivos
- [x] Refatorar `AppShell` para suportar panes redimensionáveis.
- [x] Refatorar `LibrarySidebar` para suportar redimensionamento vertical entre painéis.
- [x] Criar o componente `Viewport` em `src/components/layout`.
- [x] Criar subcomponentes em `src/components/features/viewport`:
    - [x] `ListView`: Gerencia a listagem de arquivos com Toolbar (busca, ordenação, layout).
    - [x] `ItemView`: Visualizador individual com ferramentas de manipulação visual (zoom, pan, rotate).

## 🏗️ Arquitetura e Decisões de Design

### Redimensionamento (Resizable)
- Utilização do `ResizablePanelGroup`, `ResizablePanel` e `ResizableHandle` (já implementados em `@ui`).
- **AppShell**: Divisão horizontal (Sidebar | Viewport | Inspector).
- **LibrarySidebar**: Divisão vertical (Library | Folders | Tags).

### Viewport Logic
- O `Viewport` atuará como um controlador de estado para alternar entre `ListView` e `ItemView`.
- **ListView**:
    - `ListViewToolbar`: Integrará busca (vinda do header), histórico e controles de exibição.
    - `ListViewContent`: Renderização condicional de layouts (Masonry, Grid, List). Inicialmente apenas Masonry.
- **ItemView**:
    - `ItemViewToolbar`: Navegação entre itens, controles de zoom e ferramentas de visualização.
    - `ItemViewContent`: Área de canvas/imagem com suporte a Pan, Zoom e Rotate.

### Estilização e Acessibilidade
- Estilização via arquivos CSS dedicados por componente.
- Uso estrito de tokens CSS.
- Foco em Micro-interações (hover effects, transições suaves).

## 📅 Cronograma de Implementação

### Fase 1: Fundação e Estrutura (AppShell & Sidebar)
1.  [x] Refatorar `src/layouts/AppShell.tsx` para usar `Resizable`.
2.  [x] Refatorar `src/components/layout/LibrarySidebar.tsx` para usar `Resizable` (vertical).
3.  [x] Ajustar `app-shell.css` e `library-sidebar.css`.

### Fase 2: Componente Viewport
1.  [x] Criar `src/components/layout/Viewport.tsx`.
2.  [x] Definir estados de navegação (Contexto: List vs Item).

### Fase 3: ListView & Toolbar
1.  [x] Criar `src/components/features/viewport/ListView.tsx`.
2.  [x] Criar `src/components/features/viewport/ListViewToolbar.tsx`.
3.  [x] Mover a busca de `PrimaryHeader` para `ListViewToolbar`.
4.  [x] Implementar Dropdowns de Ordenação e Layout.
5.  [x] Integrar `VirtualMasonry` como visualização padrão.

### Fase 4: ItemView & Visualizador
1.  [x] Criar `src/components/features/viewport/ItemView.tsx`.
2.  [x] Criar `src/components/features/viewport/ItemViewToolbar.tsx`.
3.  [x] Implementar lógica de manipulação visual (Zoom/Pan/Rotate).
4.  [x] Adicionar navegação Próximo/Anterior.

### Fase 5: Polimento e Testes
1.  [x] Finalizar estilização CSS.
2.  [x] Realizar auditoria de UX e Acessibilidade.
3.  [x] Rodar `checklist.py` para validação final.

## 🚀 Implementações Além do Planejado
- **Centralização de Tipos**: Criação de `src/types/index.ts` para unificar a interface `ImageItem`, eliminando duplicações e conflitos circulares entre store e utilities.
- **Infraestrutura de Scripts**: Atualização do `checklist.py` para detecção e uso automático de `python3`, resolvendo erros de ambiente em sistemas Unix-like.
- **Deteção de Bugs de TSC**: Identificação e remoção de múltiplos imports não utilizados e variáveis mortas em componentes críticos (`AssetCard`, `thumbnailStore`, etc).

## ✨ Melhorias de Engenharia
- **Reescrita do Componente Resizable**: Refatoração profunda para suportar múltiplos grupos aninhados e IDs estáveis, além de corrigir a reatividade de estilos no SolidJS (problema que impedia o redimensionamento fluído da Sidebar/Inspector).
- **Tipagem Estrita de Navegação**: Conversão forçada de IDs para `string` em todos os pontos de entrada do `useViewport`, prevenindo bugs silenciosos de "item não encontrado" por incompatibilidade `number` vs `string`.
- **Simplificação de UI**: Migração do `DropdownMenu` para uma API baseada em configurações (`items` prop), reduzindo o boilerplate e o risco de erros de renderização manual.

## 🚧 Pendências e Próximos Passos
- [ ] **Novos Layouts**: Implementar os modos `Grid` e `List` no `ListView` (atualmente apenas `Masonry` está ativo).
- [ ] **Acessibilidade**: Adicionar verificações de `prefers-reduced-motion` no CSS para animações de layout.
- [ ] **UX Audit Fixes**: Endereçar as falhas de "flat design" e "shadows" apontadas pelo `ux_audit.py`.
- [ ] **SEO Basics**: Preencher metatags e títulos dinâmicos conforme a navegação do Viewport.

## ✅ Critérios de Aceite
- [x] AppShell totalmente redimensionável sem quebras de layout.
- [x] Sidebar com divisões verticais ajustáveis.
- [x] Navegação fluida entre lista e visualização de item.
- [x] Barra de busca funcional dentro da Toolbar da lista.
- [x] Controles de zoom e rotação funcionando na visualização de item.
