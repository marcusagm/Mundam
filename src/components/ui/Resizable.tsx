import {
  Component,
  JSX,
  splitProps,
  createSignal,
  createEffect,
  createContext,
  useContext,
  Show,
  Accessor,
} from "solid-js";
import { cn } from "../../lib/utils";
import "./resizable.css";

// Context
interface ResizableContextValue {
  direction: Accessor<"horizontal" | "vertical">;
  registerPanel: (id: string, config: { defaultSize: number; minSize: number; maxSize: number }) => void;
  getPanelSize: (id: string) => Accessor<number>;
  startResize: (handleId: string, e: PointerEvent) => void;
  registerHandle: (id: string) => void;
}

const ResizableContext = createContext<ResizableContextValue>();

const useResizable = () => {
  const context = useContext(ResizableContext);
  if (!context) {
    throw new Error("ResizablePanel must be used within a ResizablePanelGroup");
  }
  return context;
};

// Panel Group
export interface ResizablePanelGroupProps extends JSX.HTMLAttributes<HTMLDivElement> {
  direction?: "horizontal" | "vertical";
  onLayout?: (sizes: number[]) => void;
  children: JSX.Element;
}

export const ResizablePanelGroup: Component<ResizablePanelGroupProps> = (props) => {
  const [local, others] = splitProps(props, [
    "class",
    "direction",
    "onLayout",
    "children",
  ]);

  const direction = () => local.direction || "horizontal";
  
  let containerRef: HTMLDivElement | undefined;

  // Panel registry
  const [panelOrder, setPanelOrder] = createSignal<string[]>([]);
  const panelConfigs = new Map<string, { defaultSize: number; minSize: number; maxSize: number }>();
  const [panelSizes, setPanelSizes] = createSignal<Map<string, number>>(new Map());

  // Handle registry
  const [handleOrder, setHandleOrder] = createSignal<string[]>([]);

  const registerPanel = (id: string, config: { defaultSize: number; minSize: number; maxSize: number }) => {
    panelConfigs.set(id, config);
    
    setPanelOrder(prev => {
      if (prev.includes(id)) return prev;
      return [...prev, id];
    });

    setPanelSizes((prev) => {
      if (prev.has(id)) return prev;
      const next = new Map(prev);
      next.set(id, config.defaultSize);
      return next;
    });
  };

  const registerHandle = (id: string) => {
    setHandleOrder(prev => {
      if (prev.includes(id)) return prev;
      return [...prev, id];
    });
  };

  const getPanelSize = (id: string): Accessor<number> => {
    return () => panelSizes().get(id) ?? 0;
  };

  const startResize = (handleId: string, e: PointerEvent) => {
    if (!containerRef) return;

    const handles = handleOrder();
    const handleIndex = handles.indexOf(handleId);
    if (handleIndex === -1) return;

    const panels = panelOrder();
    const beforePanelId = panels[handleIndex];
    const afterPanelId = panels[handleIndex + 1];
    
    if (!beforePanelId || !afterPanelId) return;

    const beforeConfig = panelConfigs.get(beforePanelId)!;
    const afterConfig = panelConfigs.get(afterPanelId)!;

    const containerRect = containerRef.getBoundingClientRect();
    const isHorizontal = direction() === "horizontal";
    const containerSize = isHorizontal ? containerRect.width : containerRect.height;

    const startPos = isHorizontal ? e.clientX : e.clientY;
    const startBeforeSize = panelSizes().get(beforePanelId) ?? 0;
    const startAfterSize = panelSizes().get(afterPanelId) ?? 0;

    const handlePointerMove = (e: PointerEvent) => {
      const currentPos = isHorizontal ? e.clientX : e.clientY;
      const deltaPx = currentPos - startPos;
      const deltaPercent = (deltaPx / containerSize) * 100;

      let newBeforeSize = startBeforeSize + deltaPercent;
      let newAfterSize = startAfterSize - deltaPercent;

      // Constraints
      if (newBeforeSize < beforeConfig.minSize) {
        newBeforeSize = beforeConfig.minSize;
        newAfterSize = startBeforeSize + startAfterSize - beforeConfig.minSize;
      } else if (newBeforeSize > beforeConfig.maxSize) {
        newBeforeSize = beforeConfig.maxSize;
        newAfterSize = startBeforeSize + startAfterSize - beforeConfig.maxSize;
      }

      if (newAfterSize < afterConfig.minSize) {
        newAfterSize = afterConfig.minSize;
        newBeforeSize = startBeforeSize + startAfterSize - afterConfig.minSize;
      } else if (newAfterSize > afterConfig.maxSize) {
        newAfterSize = afterConfig.maxSize;
        newBeforeSize = startBeforeSize + startAfterSize - afterConfig.maxSize;
      }

      setPanelSizes((prev) => {
        const next = new Map(prev);
        next.set(beforePanelId, newBeforeSize);
        next.set(afterPanelId, newAfterSize);
        return next;
      });
    };

    const handlePointerUp = () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      
      if (local.onLayout) {
        const sizes = panels.map(id => panelSizes().get(id) ?? 0);
        local.onLayout(sizes);
      }
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
    document.body.style.cursor = isHorizontal ? "col-resize" : "row-resize";
    document.body.style.userSelect = "none";
  };

  const contextValue: ResizableContextValue = {
    direction,
    registerPanel,
    getPanelSize,
    startResize,
    registerHandle,
  };

  return (
    <ResizableContext.Provider value={contextValue}>
      <div
        ref={containerRef}
        class={cn(
          "ui-resizable-group",
          `ui-resizable-group-${direction()}`,
          local.class
        )}
        {...others}
      >
        {local.children}
      </div>
    </ResizableContext.Provider>
  );
};

// Panel
export interface ResizablePanelProps extends JSX.HTMLAttributes<HTMLDivElement> {
  id: string;
  defaultSize?: number;
  minSize?: number;
  maxSize?: number;
  children: JSX.Element;
}

export const ResizablePanel: Component<ResizablePanelProps> = (props) => {
  const [local, others] = splitProps(props, [
    "class",
    "id",
    "defaultSize",
    "minSize",
    "maxSize",
    "children",
  ]);

  const context = useResizable();

  createEffect(() => {
    context.registerPanel(local.id, {
      defaultSize: local.defaultSize ?? 50,
      minSize: local.minSize ?? 0,
      maxSize: local.maxSize ?? 100,
    });
  });

  const size = context.getPanelSize(local.id);

  return (
    <div
      class={cn("ui-resizable-panel", local.class)}
      style={{
        [context.direction() === "horizontal" ? "width" : "height"]: `${size()}%`,
        "flex-shrink": 0,
        "flex-grow": 0,
      }}
      data-panel-id={local.id}
      {...others}
    >
      {local.children}
    </div>
  );
};

// Handle
export interface ResizableHandleProps extends JSX.HTMLAttributes<HTMLDivElement> {
  id?: string;
  withHandle?: boolean;
}

let handleIdCounter = 0;

export const ResizableHandle: Component<ResizableHandleProps> = (props) => {
  const [local, others] = splitProps(props, ["id", "class", "withHandle"]);
  const context = useResizable();
  
  const handleId = local.id || `handle-${++handleIdCounter}`;

  createEffect(() => {
    context.registerHandle(handleId);
  });

  const handlePointerDown = (e: PointerEvent) => {
    e.preventDefault();
    context.startResize(handleId, e);
  };

  return (
    <div
      class={cn(
        "ui-resizable-handle",
        `ui-resizable-handle-${context.direction()}`,
        local.class
      )}
      onPointerDown={handlePointerDown}
      {...others}
    >
      <Show when={local.withHandle}>
        <div class="ui-resizable-handle-bar" />
      </Show>
    </div>
  );
};
