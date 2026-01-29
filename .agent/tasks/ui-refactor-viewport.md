# Task: Refatoração da Interface e Implementação do Viewport

Este plano descreve a reorganização da interface do Elleven-Library, introduzindo redimensionamento dinâmico em todos os contextos e a criação de um sistema de Viewport robusto para navegação e visualização de arquivos.

## 🎯 Objetivos
- Refatorar `AppShell` para suportar panes redimensionáveis.
- Refatorar `LibrarySidebar` para suportar redimensionamento vertical entre painéis.
- Criar o componente `Viewport` em `src/components/layout`.
- Criar subcomponentes em `src/components/features/viewport`:
    - `ListView`: Gerencia a listagem de arquivos com Toolbar (busca, ordenação, layout).
    - `ItemView`: Visualizador individual com ferramentas de manipulação visual (zoom, pan, rotate).

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
1.  [ ] Refatorar `src/layouts/AppShell.tsx` para usar `Resizable`.
2.  [ ] Refatorar `src/components/layout/LibrarySidebar.tsx` para usar `Resizable` (vertical).
3.  [ ] Ajustar `app-shell.css` e `library-sidebar.css`.

### Fase 2: Componente Viewport
1.  [ ] Criar `src/components/layout/Viewport.tsx`.
2.  [ ] Definir estados de navegação (Contexto: List vs Item).

### Fase 3: ListView & Toolbar
1.  [ ] Criar `src/components/features/viewport/ListView.tsx`.
2.  [ ] Criar `src/components/features/viewport/ListViewToolbar.tsx`.
3.  [ ] Mover a busca de `PrimaryHeader` para `ListViewToolbar`.
4.  [ ] Implementar Dropdowns de Ordenação e Layout.
5.  [ ] Integrar `VirtualMasonry` como visualização padrão.

### Fase 4: ItemView & Visualizador
1.  [ ] Criar `src/components/features/viewport/ItemView.tsx`.
2.  [ ] Criar `src/components/features/viewport/ItemViewToolbar.tsx`.
3.  [ ] Implementar lógica de manipulação visual (Zoom/Pan/Rotate).
4.  [ ] Adicionar navegação Próximo/Anterior.

### Fase 5: Polimento e Testes
1.  [ ] Finalizar estilização CSS.
2.  [ ] Realizar auditoria de UX e Acessibilidade.
3.  [ ] Rodar `checklist.py` para validação final.

## ✅ Critérios de Aceite
- [ ] AppShell totalmente redimensionável sem quebras de layout.
- [ ] Sidebar com divisões verticais ajustáveis.
- [ ] Navegação fluida entre lista e visualização de item.
- [ ] Barra de busca funcional dentro da Toolbar da lista.
- [ ] Controles de zoom e rotação funcionando na visualização de item.
