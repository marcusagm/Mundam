Com base na análise do seu projeto (`elleven-library`) e do projeto de referência (`allusion-app`), elaborei um **Plano de Refatoração Definitivo para o Sistema de Viewport (Masonry)**.

O projeto `allusion-app` utiliza uma arquitetura extremamente robusta: ele compila código **Rust para WebAssembly (WASM)** para calcular o layout, e executa isso dentro de um **Web Worker**. Isso garante que o cálculo matemático pesado nunca bloqueie a thread principal da UI.

Para atingir o nível de "Excelência em Arquitetura" solicitado, não faremos apenas um patch; proporei uma reescrita do subsistema de visualização seguindo os princípios de **Off-Main-Thread Architecture (OMTA)** e **Spatial Partitioning**.

Como o plano é extenso, dividi em **duas partes**.

---

# Parte 1: Arquitetura, Contratos de Dados e Desacoplamento

Esta fase foca em preparar o terreno para que o novo motor de renderização funcione sem "gambiarras" e com separação clara de responsabilidades.

## 1. Visão Geral da Nova Arquitetura

Abandonaremos a abordagem síncrona atual. A nova arquitetura terá três camadas distintas:

1. **Camada de Apresentação (Main Thread / SolidJS):** Responsável *apenas* por renderizar os itens que o Worker disser que estão visíveis e gerenciar interações (cliques, drag-and-drop).
2. **Camada de Orquestração (Bridge / Store):** Gerencia a comunicação assíncrona entre o SolidJS e o Worker, mantendo um cache de posições para evitar "layout shifts".
3. **Camada de Cálculo (Web Worker):** O "cérebro". Recebe lista de itens e dimensões do container, calcula posições (x, y) e usa um **Índice Espacial** (Spatial Grid ou Quadtree) para responder instantaneamente "quais itens estão visíveis no scroll Y?".

---

## 2. Refatoração de Entidades e Interfaces

Atualmente, o `VirtualMasonry` mistura dados brutos com lógica de layout. Precisamos tipar estritamente as mensagens trocadas.

**Arquivo Sugerido:** `src/core/viewport/types.ts`

```typescript
// O que o Worker precisa saber para calcular? (Minimalismo)
export interface MasonryItemInput {
  id: string;
  width: number;
  height: number; // Aspect ratio é crucial
  pinned?: boolean; // Futuro: suporte a itens fixos
}

// Configuração do Layout
export interface MasonryConfig {
  containerWidth: number;
  columnWidth: number; // ou minColumnWidth
  gap: number;
  buffer: number; // Pixels extras para renderizar acima/abaixo
}

// O que o Worker responde?
export interface LayoutPosition {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

// Resposta completa do Worker para a UI
export interface VirtualizationResult {
  totalHeight: number;
  visibleItems: LayoutPosition[]; // Apenas o subset visível
  startIndex: number;
  endIndex: number;
}

```

---

## 3. Desacoplamento do `AssetCard` (Code Smell Crítico)

Como identificado na auditoria, o componente `AssetCard` atual consome stores globais, o que impede a virtualização eficiente (re-renders desnecessários).

**Ação:** Transformar `AssetCard` em um *Dumb Component* (Puro).

**Novo Contrato (`AssetCard.props`):**

```typescript
interface AssetCardProps {
  // Dados primitivos (NÃO passar o objeto 'Item' inteiro se possível)
  id: string;
  title: string;
  thumbnailUrl: string;
  rating: number;
  
  // Estado visual
  isSelected: boolean;
  style: JSX.CSSProperties; // Para receber transform: translate(x, y)
  
  // Callbacks (Evita importar hooks de ação dentro do card)
  onSelect: (id: string, multi: boolean) => void;
  onOpen: (id: string) => void;
  onDragStart: (e: DragEvent) => void;
}

```

**Benefício:** Com isso, podemos usar `Before` e `Memo` do SolidJS para garantir que o Card só renderize se sua posição ou estado de seleção mudar, ignorando mudanças no resto da biblioteca.

---

## 4. O Web Worker de Layout (Engine)

Aqui está o diferencial de performance. Ao invés de tentar portar o WASM do Allusion agora (que adicionaria complexidade de build Rust->WASM no seu pipeline atual), recomendo implementar um **Worker TypeScript Otimizado** com uma estrutura de dados de **Grade Espacial**.

*Por que não WASM agora?* O gargalo atual é o algoritmo O(N) no JS, não a falta de performance bruta do JS. Um worker JS bem feito (Spatial Grid) processa 100k itens em milissegundos.

**Estrutura do Worker (`src/core/viewport/masonry.worker.ts`):**

1. **State Local do Worker:** Mantém um cache de `Map<ItemId, Position>`.
2. **Spatial Grid:** Uma matriz simples onde cada "célula" (ex: 500px de altura) contém uma lista de IDs de itens que cruzam aquela área.
3. **Algoritmo de Layout:**
* Mantém um array `columnHeights` (altura atual de cada coluna).
* Para cada item, encontra a coluna mais baixa (`min(columnHeights)`).
* Coloca o item lá, atualiza a altura da coluna.
* Insere o item na **Spatial Grid**.



**Lógica de "Query" (O Scroll):**
Quando a UI manda uma mensagem `SCROLL { scrollTop: 1000, viewportHeight: 800 }`:

1. O Worker calcula quais células da Grid intersectam o intervalo `1000` até `1800` (+ buffer).
2. Concatena os IDs dessas células.
3. Envia de volta apenas as posições desses itens.

---

## 5. Plano de Execução - Fase 1

Antes de codificar o Worker complexo, precisamos preparar o terreno:

1. **Criar a pasta `src/core/viewport/**`: Centralizar toda lógica de layout aqui.
2. **Definir `types.ts**`: Copiar as interfaces definidas acima.
3. **Refatorar `AssetCard.tsx**`:
* Remover `useLibrary()`, `useSelection()`.
* Receber tudo via props.
* Criar um componente wrapper `AssetCardContainer.tsx` se precisar conectar com a store, mas o `VirtualMasonry` deve controlar isso.


4. **Criar o boilerplate do Worker**:
* Configurar o arquivo `.worker.ts` e garantir que o Vite o compile corretamente (o padrão do Vite já suporta `new Worker(..., { type: 'module' })`).


Esta é a **Parte 2** do plano de refatoração: **Implementação da "Engine" de Layout e Integração**.

Nesta etapa, vamos implementar o algoritmo de **Particionamento Espacial (Spatial Grid)** dentro de um Web Worker. Isso removerá 100% da carga de cálculo de layout e filtragem de visibilidade da thread principal, permitindo que a UI permaneça fluida (60/120 FPS) mesmo com 50.000+ itens.

---

## 1. O Algoritmo: Spatial Grid (Grade Espacial)

Ao invés de verificar cada item para saber se está na tela (O(N)), dividiremos o "mundo" virtual da rolagem em células fixas (ex: blocos de 1000px de altura).

* **Inserção:** Quando calculamos onde a imagem fica (x, y), registramos o ID dela na(s) célula(s) correspondente(s).
* **Consulta (Scroll):** Se o usuário está no pixel 5000, olhamos apenas a Célula 5 e retornamos seus itens.

---

## 2. Implementação do Worker (`src/core/viewport/masonry.worker.ts`)

Crie este arquivo. Ele será o "cérebro" isolado.

```typescript
/// <reference lib="webworker" />

// Tipos definidos na Parte 1 (importe ou defina aqui para o worker)
type LayoutItem = { id: string; aspectRatio: number };
type Position = { id: string; x: number; y: number; width: number; height: number };

// Configurações
let config = {
  columns: 4,
  columnWidth: 250,
  gap: 16,
  containerWidth: 1000,
  cellHeight: 1000 // Altura da célula da Grade Espacial
};

// --- ESTADO INTERNO DO WORKER ---
// 1. Mapa de posições exatas
const positions = new Map<string, Position>();
// 2. O índice espacial (Bucket ID -> Lista de IDs de itens)
const spatialGrid = new Map<number, string[]>();
// 3. Altura total do container (para a scrollbar)
let totalHeight = 0;
// 4. Cache da lista de itens brutos para recálculo rápido
let rawItems: LayoutItem[] = [];

self.onmessage = (e: MessageEvent) => {
  const { type, payload } = e.data;

  switch (type) {
    case 'SET_ITEMS':
      rawItems = payload;
      recalculateLayout();
      break;

    case 'RESIZE':
      // Payload: { width, minColumnWidth, gap }
      if (config.containerWidth === payload.width) return;
      config = { ...config, ...payload };
      recalculateLayout();
      break;

    case 'SCROLL':
      // Payload: { scrollTop, viewportHeight }
      const visibleData = queryVisibleItems(payload.scrollTop, payload.viewportHeight);
      self.postMessage({ type: 'VISIBLE_UPDATE', payload: visibleData });
      break;
  }
};

// --- NÚCLEO: CÁLCULO DE MASONRY + INDEXAÇÃO ---
function recalculateLayout() {
  positions.clear();
  spatialGrid.clear();
  
  // Array segurando a altura atual de cada coluna
  const columnHeights = new Array(config.columns).fill(0);
  
  // Largura real da coluna (ajustada para caber no container)
  // Lógica simplificada aqui, pode usar a sua lógica exata de resize
  const colCount = Math.floor((config.containerWidth + config.gap) / (config.columnWidth + config.gap));
  const safeCols = Math.max(1, colCount);
  const realColWidth = (config.containerWidth - (safeCols - 1) * config.gap) / safeCols;

  rawItems.forEach(item => {
    // 1. Achar a coluna mais curta
    let minColIndex = 0;
    let minHeight = columnHeights[0];
    
    for (let i = 1; i < safeCols; i++) {
      if (columnHeights[i] < minHeight) {
        minHeight = columnHeights[i];
        minColIndex = i;
      }
    }

    // 2. Calcular posição
    const x = minColIndex * (realColWidth + config.gap);
    const y = minHeight;
    const height = realColWidth / item.aspectRatio; // Altura baseada no aspect ratio

    // 3. Salvar posição
    const pos: Position = { id: item.id, x, y, width: realColWidth, height };
    positions.set(item.id, pos);

    // 4. Atualizar altura da coluna
    columnHeights[minColIndex] += height + config.gap;

    // 5. INDEXAÇÃO ESPACIAL (O Pulo do Gato)
    const startCell = Math.floor(y / config.cellHeight);
    const endCell = Math.floor((y + height) / config.cellHeight);

    for (let cell = startCell; cell <= endCell; cell++) {
      if (!spatialGrid.has(cell)) spatialGrid.set(cell, []);
      spatialGrid.get(cell)!.push(item.id);
    }
  });

  totalHeight = Math.max(...columnHeights);

  // Avisa a UI que o layout mudou (nova altura total, reinicia render)
  self.postMessage({ 
    type: 'LAYOUT_COMPLETED', 
    payload: { totalHeight } 
  });
}

// --- CONSULTA OTIMIZADA ---
function queryVisibleItems(scrollTop: number, viewportHeight: number) {
  const buffer = 1000; // Renderizar 1000px acima e abaixo para scroll suave
  const startY = Math.max(0, scrollTop - buffer);
  const endY = scrollTop + viewportHeight + buffer;

  const startCell = Math.floor(startY / config.cellHeight);
  const endCell = Math.floor(endY / config.cellHeight);

  const visibleIds = new Set<string>();

  for (let cell = startCell; cell <= endCell; cell++) {
    const idsInCell = spatialGrid.get(cell);
    if (idsInCell) {
      for (const id of idsInCell) {
        visibleIds.add(id);
      }
    }
  }

  // Mapear IDs para posições reais
  const result: Position[] = [];
  visibleIds.forEach(id => {
    const pos = positions.get(id);
    if (pos) {
      // Verificação fina final (opcional, mas bom para precisão)
      if (pos.y + pos.height > startY && pos.y < endY) {
        result.push(pos);
      }
    }
  });

  return result;
}

```

---

## 3. O Orquestrador (Frontend Service)

Agora precisamos de uma classe no SolidJS que gerencie esse worker e forneça *Signals* reativos para o componente.

**Arquivo:** `src/core/viewport/ViewportController.ts`

```typescript
import { createSignal, createRoot, onCleanup } from "solid-js";
import MasonryWorker from './masonry.worker?worker'; // Sintaxe do Vite para importar Worker

export class ViewportController {
  private worker: Worker;
  
  // Sinais Reativos para a UI
  public visibleItems = createSignal<any[]>([]); // Itens renderizados no DOM
  public totalHeight = createSignal(0); // Altura da "div fantasma" de scroll
  public isCalculating = createSignal(false);

  constructor() {
    this.worker = new MasonryWorker();
    this.setupListeners();
  }

  private setupListeners() {
    this.worker.onmessage = (e) => {
      const { type, payload } = e.data;
      
      switch (type) {
        case 'VISIBLE_UPDATE':
          // Atualiza apenas os itens que devem aparecer na tela
          // O SolidJS fará o diff eficiente
          this.visibleItems[1](payload); 
          break;
          
        case 'LAYOUT_COMPLETED':
          this.totalHeight[1](payload.totalHeight);
          this.isCalculating[1](false);
          // Força uma atualização imediata da visibilidade baseada no scroll atual
          // (Pode precisar passar o scrollTop atual aqui se não estiver sincronizado)
          break;
      }
    };
  }

  // --- API PÚBLICA ---

  public setItems(items: any[]) {
    this.isCalculating[1](true);
    // Enviar apenas dados mínimos necessários (ID e Aspect Ratio) para serialização rápida
    const lightweightItems = items.map(i => ({ 
      id: i.id, 
      aspectRatio: i.metadata?.width / i.metadata?.height || 1 
    }));
    
    this.worker.postMessage({ type: 'SET_ITEMS', payload: lightweightItems });
  }

  public handleResize(width: number) {
    this.worker.postMessage({ 
      type: 'RESIZE', 
      payload: { width, minColumnWidth: 200, gap: 16 } 
    });
  }

  public handleScroll(scrollTop: number, viewportHeight: number) {
    // Debounce não é estritamente necessário aqui se o worker for rápido,
    // mas requestAnimationFrame no componente que chama isso é recomendado.
    this.worker.postMessage({ 
      type: 'SCROLL', 
      payload: { scrollTop, viewportHeight } 
    });
  }

  public dispose() {
    this.worker.terminate();
  }
}

// Singleton exportado (ou use Context)
export const viewportController = createRoot(() => new ViewportController());

```

---

## 4. O Componente Viewport (`VirtualMasonry.tsx`)

O componente agora é "burro". Ele apenas reflete o estado do controller.

```tsx
import { onMount, onCleanup, createEffect } from "solid-js";
import { viewportController } from "../../core/viewport/ViewportController";
import { AssetCard } from "./AssetCard"; // Versão refatorada "Dumb"

export const VirtualMasonry = (props: { items: any[] }) => {
  let containerRef: HTMLDivElement | undefined;
  let scrollParentRef: HTMLDivElement | undefined; // Elemento com overflow-y: auto

  // Sincroniza itens da store com o controller
  createEffect(() => {
    viewportController.setItems(props.items);
  });

  // Resize Observer
  onMount(() => {
    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        viewportController.handleResize(entry.contentRect.width);
      }
    });
    if (containerRef) ro.observe(containerRef);

    // Scroll Listener
    // Assumindo que o scroll está no window ou num elemento pai. 
    // Adapte para seu layout (ex: .viewport-scroll-area)
    const handleScroll = () => {
        // requestAnimationFrame para não spamar o worker
        requestAnimationFrame(() => {
            const scrollTop = window.scrollY || document.documentElement.scrollTop;
            const height = window.innerHeight;
            viewportController.handleScroll(scrollTop, height);
        });
    };

    window.addEventListener('scroll', handleScroll, { passive: true });

    onCleanup(() => {
      ro.disconnect();
      window.removeEventListener('scroll', handleScroll);
      viewportController.dispose();
    });
  });

  const [visiblePositions] = viewportController.visibleItems;
  const [height] = viewportController.totalHeight;

  return (
    <div 
      ref={containerRef} 
      style={{ 
        height: `${height()}px`, // Define a altura total para a scrollbar existir
        position: 'relative',
        width: '100%' 
      }}
    >
      <For each={visiblePositions()}>
        {(pos) => {
            // Recuperar o dado completo do item se necessário (ex: URL da thumb)
            // Pode usar um Map<ID, Item> na memória principal para lookup rápido O(1)
            const itemData = props.items.find(i => i.id === pos.id); 
            if (!itemData) return null;

            return (
              <div
                style={{
                  position: 'absolute',
                  transform: `translate3d(${pos.x}px, ${pos.y}px, 0)`,
                  width: `${pos.width}px`,
                  height: `${pos.height}px`,
                  // contain: 'paint' ajuda o browser a otimizar
                  contain: 'paint layout' 
                }}
              >
                <AssetCard 
                    id={itemData.id}
                    thumbnail={itemData.thumbnail}
                    // ... outras props
                />
              </div>
            );
        }}
      </For>
    </div>
  );
};

```

---

## Próximos Passos (Plano de Execução)

1. **Criar a pasta `src/core/viewport**`.
2. **Copiar o `masonry.worker.ts**` para lá.
3. **Configurar o Vite:** Se você ainda não usa workers, o Vite geralmente lida com `?worker` automaticamente. Caso contrário, adicione a config básica no `vite.config.ts`.
4. **Implementar `ViewportController**` e conectar na `libraryStore`.
5. **Substituir o `VirtualMasonry**` atual pela versão acima.
6. **Teste de Estresse:** Carregue 10.000 placeholders e faça scroll rápido.

Se estiver satisfeito com este plano detalhado, posso ajudar a **analisar como integrar o Drag-and-Drop** neste novo sistema, já que os itens não existem no DOM quando estão fora da tela (virtualização), o que quebra bibliotecas de DnD tradicionais. Deseja abordar isso?



Esta é a **Parte 3** do plano, focada especificamente na integração do **Drag-and-Drop (DnD)** com a nova arquitetura de **Worker/Virtualização**.

### O Desafio Crítico

A implementação atual de DnD (`assetDirective.ts` e `ImageDropStrategy.ts`) depende de eventos do DOM (`ondragenter`, `ondrop`) anexados a **cada card**.
Na nova arquitetura virtualizada, **os cards fora da tela não existem no DOM**. Se o usuário tentar arrastar um item e soltar em uma área "vazia" ou virtualizada, o navegador não disparará os eventos no alvo correto, quebrando a funcionalidade.

### A Solução: DnD Baseado em Coordenadas (Coordinate-Based DnD)

Ao invés de confiar no DOM para saber "em cima de quem estou", usaremos matemática. O container principal capturará o movimento do mouse e calcularemos qual é o índice de inserção baseado nas coordenadas (X, Y) enviadas pelo Worker.

---

## 1. Adaptação da Diretiva de Origem (`assetDirective.ts`)

A diretiva atual `assetDnD` faz duas coisas: define a origem (drag source) e o destino (drop target). Precisamos separar isso.

**Ação:** Alterar a diretiva para lidar apenas com o **Drag Start**.

```typescript
// src/core/dnd/assetDirective.ts (Refatorado)

export const assetDragSource = (el: HTMLElement, accessor: () => { item: any }) => {
  const handleDragStart = (e: DragEvent) => {
    const { item } = accessor();
    
    // Configura o fantasma e os dados (mantém lógica atual)
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      // Truque: Passar o ID como JSON para recuperação síncrona se necessário
      e.dataTransfer.setData("application/json", JSON.stringify({ id: item.id }));
    }
    
    // Dispara a estratégia global (mantém compatibilidade com seu dnd-core)
    // Ex: dndStore.setDraggingItem(item);
  };

  el.setAttribute("draggable", "true");
  el.addEventListener("dragstart", handleDragStart);
  
  // Limpeza
  onCleanup(() => el.removeEventListener("dragstart", handleDragStart));
};

```

---

## 2. O Monitor de Drop no Container (`ViewportDropTarget`)

Não anexaremos mais listeners nos cards. O listener será no **Container do Masonry**.

Precisamos de uma função que converta `(MouseX, MouseY)` -> `TargetIndex`. Como o Worker tem o layout exato, podemos replicar uma lógica simplificada na thread principal ou consultar o cache de posições visíveis.

**Novo Arquivo:** `src/core/viewport/ViewportInteract.ts`

```typescript
import { ViewportController } from "./ViewportController";

export class ViewportInteract {
  constructor(private controller: ViewportController) {}

  /**
   * Calcula o índice de inserção baseado na posição do mouse.
   * Usa os itens visíveis para performance instantânea (60fps).
   */
  public getDropTarget(clientX: number, clientY: number, containerRect: DOMRect): { 
    targetId: string | null, 
    position: 'before' | 'after' 
  } {
    const relativeX = clientX - containerRect.left;
    const relativeY = clientY - containerRect.top; // Já considera o scroll se o container tiver overflow
    
    // Acessa as posições calculadas que a UI está usando atualmente
    const visibleLayout = this.controller.visibleItems[0](); 

    // Encontra o item sob o mouse (Hit Testing simples)
    // Otimização: Poderia usar uma QuadTree se fossem muitos itens VISÍVEIS, 
    // mas array.find em <50 itens é negligenciável.
    const hoveredItem = visibleLayout.find(pos => {
      return (
        relativeX >= pos.x &&
        relativeX <= pos.x + pos.width &&
        relativeY >= pos.y &&
        relativeY <= pos.y + pos.height
      );
    });

    if (!hoveredItem) return { targetId: null, position: 'after' };

    // Decide se está na metade esquerda ou direita (ou cima/baixo) do item
    // Aqui assumimos inserção lateral (típica de masonry)
    const centerX = hoveredItem.x + (hoveredItem.width / 2);
    const position = relativeX < centerX ? 'before' : 'after';

    return { targetId: hoveredItem.id, position };
  }
}

```

---

## 3. Feedback Visual (O "Drop Line")

Como os cards mudam de posição, não podemos confiar em CSS `:hover`. Precisamos de um componente de overlay que desenhe a linha azul de inserção.

**Componente:** `src/components/features/viewport/DragOverlay.tsx`

```tsx
import { createSignal } from "solid-js";

export const DragOverlay = () => {
  // Estado global ou passado via Contexto do DnD
  const [indicator, setIndicator] = createSignal<{ x: number, y: number, height: number } | null>(null);

  // Este componente renderiza uma linha absoluta sobre o grid
  return (
    <Show when={indicator()}>
      {(pos) => (
        <div 
          style={{
            position: 'absolute',
            left: `${pos().x}px`,
            top: `${pos().y}px`,
            height: `${pos().height}px`,
            width: '4px',
            'background-color': 'var(--color-primary)', // Azul do tema
            'border-radius': '2px',
            'pointer-events': 'none', // Crucial para não bloquear o drag
            'z-index': 9999
          }}
        />
      )}
    </Show>
  );
};

```

---

## 4. Integração no `VirtualMasonry.tsx`

Aqui unimos tudo. O container captura os eventos e delega para a lógica matemática.

```tsx
// Dentro de VirtualMasonry.tsx

const interact = new ViewportInteract(viewportController);

const handleDragOver = (e: DragEvent) => {
  e.preventDefault(); // Necessário para permitir o Drop
  
  if (!containerRef) return;
  const rect = containerRef.getBoundingClientRect();
  
  // 1. Calcular onde cairia
  const { targetId, position } = interact.getDropTarget(e.clientX, e.clientY, rect);
  
  if (targetId) {
    // 2. Atualizar visualmente onde a linha deve aparecer
    // (Lógica simplificada: buscar a posição exata do targetId no layout atual)
    const targetPos = viewportController.visibleItems[0]().find(p => p.id === targetId);
    if (targetPos) {
       updateDropIndicator(targetPos, position); // Função que atualiza o sinal do DragOverlay
    }
  }
};

const handleDrop = (e: DragEvent) => {
  e.preventDefault();
  const rect = containerRef.getBoundingClientRect();
  const { targetId, position } = interact.getDropTarget(e.clientX, e.clientY, rect);

  if (targetId) {
    // 3. INVOCAR A ESTRATÉGIA EXISTENTE
    // Aqui recuperamos sua lógica de src/core/dnd/strategies/ImageDropStrategy.ts
    // Mas adaptamos para passar o "targetId" explicitamente ao invés de depender do evento do DOM
    
    const strategy = new ImageDropStrategy(); // Ou injetado via DI
    strategy.handleDrop(e, { targetId, position }); 
  }
  
  clearDropIndicator();
};

return (
  <div 
    ref={containerRef}
    onDragOver={handleDragOver}
    onDrop={handleDrop}
    // ... estilos ...
  >
    <DragOverlay /> {/* Camada de feedback visual */}
    {/* ... Renderização dos items ... */}
  </div>
)

```

---

## 5. Ajuste na `ImageDropStrategy.ts`

Você precisará fazer uma pequena refatoração na sua classe de estratégia para que ela aceite o `targetId` vindo de fora, já que o `e.target` do evento nativo será o container geral (`div`), e não o card específico.

**Refatoração Sugerida:**

```typescript
// src/core/dnd/strategies/ImageDropStrategy.ts

export class ImageDropStrategy implements DropStrategy {
  // Alterar assinatura para aceitar contexto extra
  handleDrop(event: DragEvent, context?: { targetId: string, position: 'before'|'after' }) {
    
    const droppedFiles = event.dataTransfer?.files;
    
    // Caso 1: Arrastando arquivos externos (OS -> App)
    if (droppedFiles && droppedFiles.length > 0) {
       // ... lógica de importação (mantém igual) ...
       // Se tiver context.targetId, você sabe em qual pasta/posição importar!
       return;
    }

    // Caso 2: Reordenando itens internos
    const draggedItemId = ... // recuperar do dndStore ou dataTransfer
    const targetItemId = context?.targetId;

    if (draggedItemId && targetItemId) {
       // Executar lógica de reordenação no Backend/Store
       libraryStore.reorderItem(draggedItemId, targetItemId, context.position);
    }
  }
}

```

---

## Resumo do Plano de Migração (DnD)

1. **Limpar:** Remover a lógica de `drop` e `dragenter` da diretiva `assetDnD` atual. Deixar ela só cuidar do `dragstart` (visual do fantasma).
2. **Centralizar:** Implementar o `ViewportInteract` para calcular colisão matemática (Mouse X/Y -> Item ID).
3. **Visualizar:** Criar o componente `DragOverlay` para desenhar a linha de inserção, já que não podemos usar bordas CSS nos elementos virtuais.
4. **Conectar:** Ligar o evento `onDrop` do container principal (`VirtualMasonry`) à sua `ImageDropStrategy`, passando o ID calculado manualmente.

Este plano garante que o DnD continue funcionando com a precisão do mouse, mas com a performance da virtualização, sem depender da existência física dos elementos DOM.