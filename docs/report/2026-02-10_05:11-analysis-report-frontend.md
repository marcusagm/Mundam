# Relatório de Análise Técnica: Frontend (Mundam)
> **Escopo**: Diretório `src/` (SolidJS + Vite + TypeScript)
> **Data**: 10 de Fevereiro de 2026

## 1. Arquitetura e Estrutura de Diretórios

A organização do frontend segue uma estrutura robusta e escalável, baseada em domínios de funcionalidade e separação entre UI "pura" e componentes de negócio.

### Estrutura
*   **`core/`**: O "cérebro" da aplicação. Contém a lógica de estado (`store`), hooks globais, sistema de input (atalhos) e configurações do Tauri.
*   **`components/ui/`**: Biblioteca de componentes visuais agnósticos (botões, sliders, modais). Atua como um "Design System" interno.
*   **`components/features/`**: Componentes de negócio que compõem as telas principais (`itemview`, `library`, `settings`).
*   **`components/layout/`**: Estrutura macro da página (`Sidebar`, `Viewport`, `Inspector`).

**Avaliação**: A estrutura é excelente. Separa claramente responsabilidades, facilitando a manutenção. Novos desenvolvedores conseguem intuir onde encontrar cada peça do código.

---

## 2. Gerenciamento de Estado

O projeto utiliza o **SolidJS Store** (`createStore`) para estado global, o que é a escolha nativa e mais performática para o framework.

### Pontos de Destaque
*   **Stores Desacoplados**: `libraryStore`, `filterStore`, `videoStore` separam bem os domínios.
*   **Pattern de Actions**: A exportação de objetos `actions` junto com o estado (ex: `filterActions`) encapsula a lógica de mutação, similar ao padrão Redux/Pinia, mantendo os componentes limpos.
*   **Persistência**: O `filterStore` implementa persistência manual no `localStorage` e um sistema de **Histórico (Undo/Redo)** para navegação, o que é um recurso avançado de UX.
*   **Desempenho**: O uso de `reconcile` no `libraryStore` demonstra preocupação em minimizar re-renderizações ao atualizar grandes listas de assets.

**Ponto de Atenção**: Vimos anteriormente que o `libraryStore` contém lógica pesada de filtragem de árvore. Embora o Store seja o lugar de "lógica", cálculos intensivos de CPU devem ser movidos para o Rust (Backend) ou Web Workers para evitar travar a UI principal.

---

## 3. Design System e Estilização

A abordagem de estilização é moderna e alinhada com as tendências de 2025/2026.

### Tecnologias
*   **CSS Nativo Moderno**: Uso extensivo de Variáveis CSS (`var(--...)`).
*   **OKLCH Colors**: O arquivo `tokens.css` revela o uso do espaço de cor OKLCH, garantindo uma paleta perceptualmente uniforme e acessível, superior ao HSL/RGB tradicional.
*   **Scoping**: O uso de classes com prefixos (ex: `.ui-video-*`) em arquivos CSS dedicados (ex: `video-player.css`) sugere uma metodologia BEM ou similar para evitar conflitos, embora CSS Modules pudesse oferecer garantia de isolamento automatizada.

### Qualidade Visual (Aesthetics)
*   **Atenção aos Detalhes**: Classes como `ui-video-badge-live` e animações de pulso no player de vídeo (`ui-video-center-action-pulse`) mostram cuidado com a experiência do usuário ("Wow Factor").
*   **Theming**: Suporte nativo a temas (Dark/Light) via `[data-theme]` e variáveis CSS.

---

## 4. Análise de Componentes Chave

### 4.1 `VideoPlayer.tsx`
Um componente complexo e feature-rich.
*   **✅ Pontos Fortes**:
    *   Suporte híbrido (HLS.js + Nativo).
    *   UI Customizada completa (Seekbar com preview, controles de volume, fullscreen).
    *   Tratamento de erros e retry automático para transcoding.
    *   Acessibilidade básica (`tabindex`, atalhos de teclado).
*   **⚠️ Melhoria**: O arquivo é grande (~700 linhas). Extrair a "Barra de Controles" e a "Seekbar" para sub-componentes (`VideoControls.tsx`, `VideoSeekbar.tsx`) melhoraria a legibilidade.

### 4.2 `ItemView.tsx`
O orquestrador de visualização.
*   **✅ Pontos Fortes**:
    *   Uso de `Switch/Match` para renderização condicional eficiente.
    *   **Sistema de Atalhos**: A integração com `createConditionalScope` permite atalhos que só funcionam quando o visualizador está aberto, evitando conflitos com o resto da app.
    *   **UX**: Feedback visual de carregamento (`Loader`) e transições suaves.

### 4.3 `ListView.tsx` & Virtualização
*   A presença de `VirtualMasonry`, `VirtualGridView` e `VirtualListView` confirma que o projeto está preparado para lidar com bibliotecas de milhares de itens sem degradar a performance do DOM.

---

## 5. Código e Boas Práticas (Clean Code)

*   **TypeScript**: Tipagem forte utilizada consistentemente (`interface`, `enums` importados do backend ou definidos localmente).
*   **Nomes Significativos**: Variáveis e funções com nomes claros (`navigate`, `toggleFlipH`, `isTranscoding`).
*   **Signals e Effects**: Uso correto das primitivas do SolidJS. `createEffect` é usado com parcimônia para sincronização (ex: atrelar HLS ao elemento video), evitando "Effect Chains" desnecessários.

---

## 6. Conclusão do Frontend

O frontend do Mundam é de **nível profissional**. Ele não se parece com um site encapsulado, mas sim com uma aplicação desktop nativa.

*   **Arquitetura**: ⭐⭐⭐⭐⭐ (Excelente separação)
*   **UX/UI**: ⭐⭐⭐⭐⭐ (Moderno, responsivo, atalhos de teclado)
*   **Código**: ⭐⭐⭐⭐½ (Limpo, tipado, apenas recomendação de refatoração para componentes gigantes como VideoPlayer)
*   **Performance**: ⭐⭐⭐⭐⭐ (SolidJS + Virtualização + Stores otimizados)

**Recomendação Principal**: Manter o rigor na separação de responsabilidades. À medida que o app cresce, resistir à tentação de colocar lógica de negócio complexa (filtros pesados, processamento de dados) no thread JS da UI.
