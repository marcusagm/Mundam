# PLAN: File Inspector Overhaul

Refatoração completa do `FileInspector` para um sistema modular, orquestrado e rico em informações, suportando múltiplos tipos de mídia e metadados detalhados.

## 1. Contexto e Objetivos
- **Orquestração**: `FileInspector` decidirá qual sub-componente renderizar com base na seleção (0, 1 ou múltiplos arquivos).
- **Tipos Suportados**: Imagem, Áudio, Vídeo e Coletivo (Multi-seleção).
- **Modularidade**: Extração de lógica e estilos para `src/components/features/inspector/`.
- **UX**: Uso de Accordions para organizar grupos de informações, sistema de rating e campo de observações.
- **Visual**: Fim do CSS inline, uso estrito de `tokens.css` e extração de estilos do `global.css`.

## 2. Estratégia de Componentização (`src/components/features/inspector/`)

### 2.1 Componentes de Orquestração
- **`FileInspector.tsx`**: Ponto de entrada. Gerencia o estado de seleção e roteia para os sub-inspectors.
- **`MultiInspector.tsx`**: Exibição para quando > 1 arquivo estiver selecionado.

### 2.2 Componentes de Visualização (Preview)
- **`InspectorPreview.tsx`**: Wrapper que seleciona entre:
    - `ImagePreview`: Miniatura com zoom.
    - `AudioPreview`: Player de áudio minimalista.
    - `VideoPreview`: Player de vídeo minimalista
    - `MultiPreview`: Pilha de itens (deck effect).

### 2.3 Componentes de Dados (Metadata)
- **`CommonMetadata.tsx`**: Informações universais (Nome, Tamanho, Rating, Datas, Observação).
- **`InspectorTags.tsx`**: Gestão de tags (baseado no `TagInput` existente).
- **`AdvancedMetadata.tsx`**: Visualização bruta de dados (EXIF, JSON, etc.) em modo colapsável.
- **Especializados**:
    - `ImageMetadata`: Resolução, Perfil de cor.
    - `AudioMetadata`: Bitrate, Sample Rate, Canais.
    - `VideoMetadata`: Codec, Frame rate, Resolução.

### 2.4 Utilitários de UI (Reusable)
- **`Accordion.tsx`** (@[src/components/ui]): Componente completo, acessível e reutilizável para seções colapsáveis em toda a app.
- **`StarRating.tsx`** (@[src/components/features/inspector]): Sistema de 5 estrelas interativo.

## 3. Breakdown de Tarefas

### Fase 1: Fundação, Estilos e UI Core
- [x] Criar estrutura de pastas em `src/components/features/inspector/`.
- [x] Implementar **`Accordion.tsx`** em `src/components/ui` com:
    - [x] Lógica de acessibilidade (WAI-ARIA: `aria-expanded`, `aria-controls`).
    - [x] Suporte a ícones, labels personalizados e múltiplas variantes.
    - [x] Transições suaves de abertura/fechamento.
- [x] Criar `inspector.css` base e arquivos CSS específicos para cada componente.
- [x] Limpar `global.css` removendo estilos que agora pertencem ao Inspector.

### Fase 2: Componentes Core (Foco em Acessibilidade)
- [x] Implementar `CommonMetadata` com suporte a edição de Notas e Rating (teclado e leitor de tela).
- [x] Implementar `StarRating`.
- [x] Migrar `TagInput` para `InspectorTags`.

### Fase 3: Especialização por Tipo
- [x] Criar `ImageInspector`: Preview + Metadados específicos.
- [x] Criar `AudioInspector`: Player funcional + Metadados.
- [x] Criar `VideoInspector`: Mini-player + Metadados.
- [x] Criar `MultiInspector`: Implementar o efeito "Deck of cards" para múltiplas seleções.

### Fase 4: Orquestração e Integração
- [x] Refatorar `FileInspector.tsx` para usar o novo motor de roteamento.
- [ ] Implementar lógica de persistência para Rating e Observações no `appStore` / Backend.
- [ ] Adicionar suporte a `AdvancedMetadata` para exibir dados EXIF brutos. (Futuro)

## 4. Critérios de Verificação
- [x] O Inspector detecta automaticamente o tipo de arquivo e muda o player/miniaturas.
- [x] Ao selecionar múltiplos arquivos, o preview mostra o contador e a pilha de imagens.
- [x] Todas as seções podem ser colapsadas via Accordion.
- [x] O rating de estrelas atualiza o estado/banco de dados.
- [x] Nenhum estilo inline remanescente nos novos componentes.

---
**Status**: Concluído ✅
