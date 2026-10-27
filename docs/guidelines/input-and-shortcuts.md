# ⌨️ Input & Shortcut System Architecture

**Mundam** possui um sistema robusto de gerenciamento de atalhos e inputs (teclado e, posteriormente, gestos/mouses) localizado em `src/core/input`. Este documento define de maneira exaustiva os padrões arquiteturais, mecânicas de segurança e as guidelines de uso e acessibilidade que **devem ser estritamente seguidas** na implementação de novos recursos.

---

## 🏗️ 1. Arquitetura e Filosofia do Sistema

O sistema foi desenhado para resolver três grandes categorias de problemas em UI Complexas (como painéis com Viewports, Modais, e Inspetores):

1.  **Conflitos Globais vs Locais:** Se o atalho "Delete" remove um item na visualização, ele não pode disparar enquanto o usuário estiver dentro de um Modal deletando um texto num formulário.
2.  **Acessibilidade Semântica (A11y):** Eventos de teclado disparados para navegação não podem sequestrar comportamentos nativos do Browser esperados por leitores de tela em elementos interativos (`<button>`, `<a>`, `<select>`).
3.  **Configurabilidade:** O usuário deve ser capaz de re-mapear atalhos de ações do aplicativo ("Copiar", "Layout em Grid", "Abrir Ferramenta") de forma centralizada.

O núcleo de Input se divide em:
-   **Store Reativa (`inputStore` / `shortcutStore`):** Solid.js Signals mantendo estado do que está pressionado, qual é o escopo atual, e a lista de atalhos registrados.
-   **Dispatcher (`dispatcher.ts`):** O "Juiz" central. Recebe um token físico, checa se as regras de Escopo / Foco e prioridades batem, e faz o `Emmit`.
-   **Primitives (`createShortcut`, `createInputScope`, etc.):** Hooks declarativos para plugar componentes visuais com o sistema sem vazar a lógica do _event listener_ bruto.

---

## 🛡️ 2. O Sistema de Escopos (`InputScope`)

Escopos são "camadas virtuais" que definem qual região do App está ativamente ouvindo comandos. 

### Padrão de Prioridades (`SCOPE_PRIORITIES`)
As prioridades (definidas em `types.ts`) variam de 0 a 10000+. Quanto maior o número, mais preferência o escopo tem:
-   `global (0)`: Atalhos de todo o aplicativo (ex: Mod+K para Busca). Sempre os últimos a serem validados.
-   `viewport (10)`: Típicos atalhos de navegação do grid centralização (Setas, Enter, Espaço).
-   `image-viewer (50)`: Tela cheia, sobrepõe as funções convencionais do grid.
-   `editing (1000)`: Editores em linha, re-nomeações (evita que atalhos de tela rodem quando está escrevendo).
-   `search (1100)`: O popover de busca global.
-   `modal (1200)`: Diálogos flutuantes. Os modais bloqueiam escopos de fundo.

### `blockLowerScopes` (Bloqueio de Camadas Inferiores)
Ao empilhar um escopo (usando `createConditionalScope`), você pode passar `blockLowerScopes: true`.
Se um *Modal* (`prioridade 1200`) é ativado bloqueando o fundo, atalhos de `viewport` (`prioridade 10`) **não serão executados**, não importa qual tecla seja pressionada. 

> **Regra de Ouro (Modais e Menus):** 
> Todo e qualquer novo elemento de "Overlay" (Menus dropdowns profundos, Pickers, Diálogos) DEVE iniciar seu próprio sub-escopo reativo passando a flag de isolamento `true`.

```tsx
// Exemplo em um DropdownMenu
import { createConditionalScope } from '@/core/input';

const meuMenu = () => {
    // Escopo temporário com prioridade bem alta, bloqueando os botões da Viewport ao fundo
    createConditionalScope('menu-layer', () => isOpen(), 1500, true);
    
    // Suas implementações de teclas agora acontecem limpas no escopo 'menu-layer'
}
```

---

## 🧑‍💻 3. Como Declarar Atalhos

### Regra 1: Macro-Ações vão no `defaults.ts` (Application Macros)
Ações visíveis ao usuário e que podem ser **remapeadas no futuro** por ele DEVEM ser cadastradas apenas através da Store de Defaults da Aplicação (Configurações).

> **Atenção (Modificador Multiplataforma `Mod`):**
> Sempre utilize `Mod` para combinações principais (ex: `Mod+KeyK`, `Mod+KeyB`). O sistema normaliza automaticamente `Mod` para `Meta` (`⌘ Command`) no macOS e para `Ctrl` (`Control`) no Windows e Linux, evitando conflitos com a tecla nativa `Win` do Windows.

1. Abra `src/core/input/store/defaults.ts`
2. Construa a Entidade exportando um "command" com sua Categoria associada.

```typescript
    {
        name: 'Abrir Painel XYZ',
        description: 'Abre a ferramenta XYZ do usuário',
        keys: 'Mod+Shift+Z',
        scope: 'global',
        command: 'app:tool-xyz',  // O NOME do comando 
        category: 'Workspace'
    }
```

3. No Seu Componente Solid.js, você responde ao comando declarado:
```tsx
import { useCommand } from '@/core/input';

const PainelXYZ = () => {
   useCommand('app:tool-xyz', () => togglePainel());
}
```

### Regra 2: Comandos Temporários Locais / Navegação Interna de Widget usam `createShortcut` / `useShortcuts`

Quando você constrói um componente de interface de baixo nível (como um seletor de Tabela, ou um modal, ou um Slider) você não deve forçar as setas de navegação desse elemento como um mapeamento de atalho do usuário final em configurações globais.

Use a primitiva `createShortcut` **COM A FLAG `system: true`** para manter o escopo silencioso na árvore do DOM e na Settings Store:

```tsx
import { createShortcut } from '@/core/input';

export const MyInteractiveWidget = (props) => {
    // Atalho invisível ao painel do usuário final, roda apenas nesse escopo
    createShortcut({
        keys: 'Escape',
        scope: 'my-widget-scope',
        system: true, // 🚨 MANDATÓRIO para atalhos não-globais de interações de UI nativas do App!
        action: () => props.onClose()
    });
}
```

---

## ♿ 4. Padrões de Acessibilidade e Bloqueio de Evento (O FATOR CHAVE)

Ao se construir listeners via o sistema `inputService`, temos que ter extrema cautela com os focos HTML (`TAB`).

### 4.1 A Flag `ignoreInputs`
Todos os atalhos por padrão (quando declarados globalmente no core ou omitidos nos params) herdam o comportamento `ignoreInputs: true`.

*O que significa?* Se o atributo `document.activeElement` for um `<input>`, `<textarea>`, `<select>` ou possuir `contenteditable`, **nenhum atalho vai funcionar**.

Se você inventou um atalho global para `Mod+S` para Salvar o sistema, **você ainda quer** que ele funcione mesmo que o usuário esteja digitando numa caixa de busca. Nesse caso você deve explicitamente revogar o ignore:

```tsx
   createShortcut({
       keys: 'Mod+KeyS',
       ignoreInputs: false, // Quero salvar mesmo que eu esteja digitando nome do projeto!
       action: () => saveApplication()
   })
```

### 4.2 Botões Web Nativos, Espaço / Enter vs Escopos (Interactive Elements)
A regra de acessibilidade mais crítica é a "Tecla Tab + Enter".

O Dispatcher global do Mundam é instruído através de uma ponte segura `isInteractiveFocused()`:
**O Dispatcher abortará qualquer atalho associado as teclas "Enter" e "Space" se o framework detectar que o DOM Físico do usuário está parado em cima de um Interativo padrão (`<button>`, `<a>`, `<summary>` ou `[role="button|link|checkbox"]`).**

Esse mecanismo garante que se a sua Viewport tem o comando:
```typescript
 { name: 'Open Item', keys: 'Enter', scope: 'viewport', command: 'viewport:open' }
```
Eles nunca roubarão o Enter de um simples `<Button onAction={() => alert('X')}>` focado acidentalmente ou propositadamente por teclado na árvore do app. O Dispatcher ignorará o `shortcut` da Viewport em benefício ao comportamento Click padrão do `Elemento HTML`.

**Consequência de Design:** Se você estiver fazendo um componente super customizado estilo tela mágica (ex: HTML cego, Canvas puro como o FileInspector) que precise consumir a tecla "Espaço", certifique-se que você não coloque acidentalmente um `tabindex=0` ou roles ARIA de 'button' atrelados incorretamente envolta dele de modo que faça o dispatcher assumir que aquilo era um botão normal HTML esperando ação via Space.

---

## 🚀 5. Exemplo Completo Prático (UI Widget Perfeito)

A anatomia perfeita de um novo Popover interativo (ex: Um menu de Seleção de Etiquetas complexo gerado na tela):

```tsx
import { Component, createSignal, Show, createMemo } from 'solid-js';
import { untrack } from 'solid-js';
import { createConditionalScope, useShortcuts } from '@/core/input';

export const MyListSelectorPopup: Component<{isOpen: () => boolean}> = (props) => {
    // 1. Defina um Identificador único se este componente puder abrir várias vezes 
    // ou se renderizar multiplamente evitando cruzamento na store.
    const customInstancePrefix = createMemo(() => `popup-menu-${Math.random()}`);
    const scopeId = untrack(() => customInstancePrefix());

    // 2. Trave o escopo garantindo isolamento total do que tá debaixo 
    createConditionalScope(scopeId, props.isOpen, 1300, true); // true = BlockLowerScopes!

    // 3. Cadastre a interface do widget
    useShortcuts([
        {
            keys: 'ArrowDown',
            scope: scopeId,
            system: true, // 🚨 Para atalhos de navegação do UI não pularem no setting panel de Atalhos Globais
            enabled: props.isOpen,
            preventDefault: true, // Impedir o scroll da página, 
            action: () => MoveLista(1)
        },
        {
            keys: 'Enter',
            scope: scopeId,
            system: true,
            enabled: props.isOpen,
            preventDefault: true, 
            action: () => SelectCurrent() 
        },
        {
            keys: 'Escape',
            scope: scopeId,
            system: true,
            enabled: props.isOpen,
            action: () => CloseList()
        }
    ]);

    return (
       // Importante: Manter tabindex="-1" e focus traps no root para 
       // evitar perda do controle de acessibilidade!
       <div role="dialog" tabindex={-1}>
          // Listagens e Componentizações
       </div>
    )
}
```

---

## 📚 6. Resumo Rápido

- Ação exportável para o Usuário Modificar -> Declarar apenas no Array Global em `store/defaults.ts` e despachar um Command `app:nome:comando`.
- Menu suspenso ou janela modal sobrepondo outras -> `createConditionalScope` marcando `blockLowerScopes: true`.
- Elementos Navegacionais puros codificados localmente via `createShortcut` -> SEMPRE defina a configuração `{ system: true }` no payload de opções para que o atalho seja invisível ao "painel de gerenciamento do usuário".
- Está codando um botão ou Checkbox padrão e está preocupado se um atalho vai roubar sua ação no Enter/Space? -> Não se preocupe. Contanto que use tags semânticas (ou atributos `role="button"`), o core de atalhos (`shouldYieldToInteractive()`) protegerá seu HTML padrão contra concorrência e garantirá ativação do Handler de Cliques.
