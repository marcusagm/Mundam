# PLAN: Modal Overhaul

Implementação de um sistema de modais robusto, acessível (WAI-ARIA) e altamente flexível, seguindo padrões de excelência em arquitetura de componentes SolidJS.

## 1. Contexto e Requisitos
- **Arquitetura**: Separação em cabeçalho, corpo e rodapé. Uso de CSS separado (`modal.css`).
- **Variantes**: `sm` (300px), `md` (500px), `lg` (800px), `full` (100% viewport).
- **Acessibilidade**: Focus trapping, WAI-ARIA roles, fechar com `Escape`.
- **UX**: Backdrop com blur, scroll interno apenas no corpo, suporte a modais empilhados (stacking).
- **Configurabilidade**: Prop para desativar fechamento via clique no overlay.

## 2. Breakdown de Tarefas

### Fase 1: Design System & Styling (`modal.css`)
- [x] Definir variáveis de animação (`scale-in`, `fade-in`).
- [x] Criar classes para as variantes de tamanho.
- [x] Implementar overlay com `backdrop-filter: blur(8px)` e `rgba`.
- [x] Configurar layout flexbox para garantir que o Header/Footer sejam fixos e o Body tenha `overflow-y: auto`.

### Fase 2: Core Architecture (`Modal.tsx`)
- [x] Implementar `ModalProvider` (opcional) ou lógica de portal para gerenciar a stack.
- [x] Criar sub-componentes: `Modal.Header`, `Modal.Body`, `Modal.Footer`.
- [x] Implementar lógica de acessibilidade:
    - [x] `useFocusTrap`: Gancho para manter o foco dentro do modal.
    - [x] Event Listener Global para a tecla `Escape`.
    - [x] Lógica para impedir o scroll do `body` quando o modal estiver aberto.

### Fase 3: Refatoração de Componentes Existentes
- [x] Atualizar `PromptModal` e `ConfirmModal` para a nova estrutura.
- [x] **TagDeleteModal.tsx**: Migrar para a nova arquitetura e remover estilos inline.
- [x] Testar comportamento de "clique fora" configurável.

### Fase 4: Especialização
- [x] Garantir que o `Full Screen` cubra 100% da viewport sem bordas indesejadas.
- [x] Validar suporte a múltiplos modais (o último no DOM deve ter precedência de foco).

## 3. Atribuição de Agentes
- **Primary Agent**: `frontend-specialist` (UI/UX e Arquitetura de Componentes).
- **Reviewer**: `security-auditor` (Validação de acessibilidade e segurança de DOM).

## 4. Critérios de Verificação
- [x] Ao abrir um modal, o conteúdo de fundo fica borrado.
- [x] O foco é levado para o primeiro elemento interativo do modal e não sai dele via `Tab`.
- [x] Pressionar `Escape` feche o modal (a menos que configurado o contrário).
- [x] Em modais longos, o cabeçalho e rodapé permanecem visíveis enquanto o corpo rola.
- [x] A variante `full` ocupa 100% da tela em todos os eixos.

---
**Status**: Concluído ✅
