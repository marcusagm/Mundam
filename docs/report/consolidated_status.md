# Relatório Consolidado de Pendências e Status do Projeto
Data: 2026-02-02

Este relatório exclui itens já implementados e consolida todas as pendências ativas identificadas nos planos de implementação (`docs/plans`), na revisão de código (`docs/code-review.md`) e no roteiro de funcionalidades (`docs/idea/features.md`).

## 1. Pendências Técnicas Críticas (Backend & Arquitetura)

### Refatoração e Robustez
*   [x] **Sincronização do Indexador (`indexer/metadata.rs`):** O indexador ainda utiliza lógica simplificada para detecção de formato. Identificado em `code-review.md`.
    *   *Ação:* Migrar para `crate::formats::FileFormat::detect` para garantir a "Fonte Única da Verdade" (UMDS).
*   [x] **Tratamento de Erros no Worker de Thumbnails (`thumbnail_worker.rs`):** Identificado em `code-review.md`.
    *   *Ação:* Implementar mecanismo de "Poison Pill" ou contagem de retentativas no banco de dados para evitar loops infinitos em arquivos corrompidos.
*   [x] **Renderização de SVGs (`thumbnails/mod.rs`):** Estratégia definida mas não implementada logicamente.
    *   *Ação:* Implementar renderização real via Webview oculta ou biblioteca específica (resvg/librsvg).
*   [ ] **Extração de Assets de Design (.ai, .eps):**
    *   *Ação:* Melhorar detecção e extração de preview (via PDF stream) ao invés de usar ícone genérico.

### Performance
*   [x] **Otimização de Leitura de Header:** Evitar abrir o arquivo duas vezes (uma para detecção, outra para processamento).
*   [x] **Otimização do Banco de Dados:** Implementar rotina de `VACUUM` e `ANALYZE` para manutenção periódica do SQLite.

## 2. Funcionalidades Pendentes (Core & UX)

### Ingestão Avançada
*   [ ] **Web Clipper / Extensão de Navegador:** (Planejado em `features.md`)
*   [ ] **Integração com Clipboard:** Colar imagens (Ctrl+V) diretamente na biblioteca.
*   [ ] **Deep OS Integration:** Atalhos globais para screenshot.

### Organização Inteligente
*   [ ] **Busca Avançada (Interface Visual):** O backend (`search_logic.rs`) suporta grupos AND/OR, mas falta uma UI "Query Builder" para o usuário final.
*   [ ] **Busca e Análise de Cores:** Extração de paleta na indexação e filtros de busca por cor.
*   [ ] **Deduplicação:** Detecção de arquivos duplicados.
*   [ ] **Fuzzy Search:** Tolerância a erros de digitação na busca.

### Visualização e Inspeção
*   [x] **Visualização 3D Real:** Renderização interativa de múltiplos formatos (.obj, .fbx, .blend, .dae, etc).
*   [x] **Preview de Fontes:** Digitação de texto customizado para testar fontes.
*   [ ] **Video Player Avançado:** Scrubbing (hover), playback speed, anotações.

### Gestão em Massa
*   [ ] **Batch Operations:** UI para renomear, mover ou editar tags de milhares de arquivos simultaneamente.

## 3. Infraestrutura e Nuvem
*   [ ] **Cloud Sync:** Sincronização com GDrive/Dropbox/etc.
*   [ ] **Exportação Portátil:** Exportar coleções com metadados auto-contidos.

## 4. Interface e Personalização
*   [ ] **Temas Avançados:** Gerenciamento completo de CSS/Temas (além do status bar toggle).
*   [ ] **Localização (i18n):** Tradução da interface.

## 5. Status de Funcionalidades Recentes (Implementadas)

Abaixo, itens que foram verificados como ENTREGUES e não constam mais como pendências:

*   [x] **Taxonomia Hierárquica:** (Backend e Frontend completos).
*   [x] **Visualização Recursiva de Pastas:** (Comandos SQL `WITH RECURSIVE` ativos).
*   [x] **Otimização de Thumbnails:** (Paralelismo Rayon ativo).
*   [x] **Ordenação Global (Global Sorting):** (Implementado no Store e Queries).
*   [x] **Modal de Configurações:** (Refatorado).
*   [x] **Layout Masonry Virtualizado:** (Implementado).

---
Este relatório deve ser utilizado para priorizar os próximos Sprints. Recomenda-se foco imediato nas **Pendências Técnicas Críticas** antes de iniciar novas features de Ingestão.
