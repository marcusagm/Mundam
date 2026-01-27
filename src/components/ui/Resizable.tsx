import {
  Component,
  JSX,
  splitProps,
  createSignal,
  createEffect,
  onCleanup,
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
  registerPanel: (id: string, defaultSize: number, minSize?: number, maxSize?: number) => void;
  getPanelSize: (id: string) => Accessor<number>;
  startResize: (handleIndex: number, e: PointerEvent) => void;
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

/**
 * ResizablePanelGroup for creating resizable panels.
 * 
 * @example
 * <ResizablePanelGroup direction="horizontal">
 *   <ResizablePanel defaultSize={30}>
 *     <Sidebar />
 *   </ResizablePanel>
 *   <ResizableHandle />
 *   <ResizablePanel defaultSize={70}>
 *     <Content />
 *   </ResizablePanel>
 * </ResizablePanelGroup>
 */
export const ResizablePanelGroup: Component<ResizablePanelGroupProps> = (props) => {
  const [local, others] = splitProps(props, [
    "class",
    "direction",
    "onLayout",
    "children",
  ]);

  const direction = () => local.direction || "horizontal";
  
  let containerRef: HTMLDivElement | undefined;

  // Panel state registry
  const panelConfigs: Map<string, { defaultSize: number; minSize: number; maxSize: number }> = new Map();
  const [panelSizes, setPanelSizes] = createSignal<Map<string, number>>(new Map());

  const registerPanel = (id: string, defaultSize: number, minSize = 0, maxSize = 100) => {
    panelConfigs.set(id, { defaultSize, minSize, maxSize });
    setPanelSizes((prev) => {
      const next = new Map(prev);
      next.set(id, defaultSize);
      return next;
    });
  };

  const getPanelSize = (id: string): Accessor<number> => {
    return () => panelSizes().get(id) ?? 50;
  };

  const startResize = (handleIndex: number, e: PointerEvent) => {
    e.preventDefault();
    
    if (!containerRef) return;

    const panelIds = Array.from(panelConfigs.keys());
    const beforePanelId = panelIds[handleIndex];
    const afterPanelId = panelIds[handleIndex + 1];
    
    if (!beforePanelId || !afterPanelId) return;

    const beforeConfig = panelConfigs.get(beforePanelId)!;
    const afterConfig = panelConfigs.get(afterPanelId)!;

    const containerRect = containerRef.getBoundingClientRect();
    const isHorizontal = direction() === "horizontal";
    const containerSize = isHorizontal ? containerRect.width : containerRect.height;

    const startPos = isHorizontal ? e.clientX : e.clientY;
    const startBeforeSize = panelSizes().get(beforePanelId) ?? 50;
    const startAfterSize = panelSizes().get(afterPanelId) ?? 50;

    const handlePointerMove = (e: PointerEvent) => {
      const currentPos = isHorizontal ? e.clientX : e.clientY;
      const delta = ((currentPos - startPos) / containerSize) * 100;

      let newBeforeSize = startBeforeSize + delta;
      let newAfterSize = startAfterSize - delta;

      // Clamp to min/max
      if (newBeforeSize < beforeConfig.minSize) {
        newBeforeSize = beforeConfig.minSize;
        newAfterSize = startBeforeSize + startAfterSize - beforeConfig.minSize;
      }
      if (newBeforeSize > beforeConfig.maxSize) {
        newBeforeSize = beforeConfig.maxSize;
        newAfterSize = startBeforeSize + startAfterSize - beforeConfig.maxSize;
      }
      if (newAfterSize < afterConfig.minSize) {
        newAfterSize = afterConfig.minSize;
        newBeforeSize = startBeforeSize + startAfterSize - afterConfig.minSize;
      }
      if (newAfterSize > afterConfig.maxSize) {
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
      document.removeEventListener("pointermove", handlePointerMove);
      document.removeEventListener("pointerup", handlePointerUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      
      const sizes = Array.from(panelSizes().values());
      local.onLayout?.(sizes);
    };

    document.addEventListener("pointermove", handlePointerMove);
    document.addEventListener("pointerup", handlePointerUp);
    document.body.style.cursor = isHorizontal ? "col-resize" : "row-resize";
    document.body.style.userSelect = "none";
  };

  const contextValue: ResizableContextValue = {
    direction,
    registerPanel,
    getPanelSize,
    startResize,
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
  id?: string;
  defaultSize?: number;
  minSize?: number;
  maxSize?: number;
  children: JSX.Element;
}

let panelCounter = 0;

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
  const panelId = local.id || `panel-${++panelCounter}`;

  // Register panel on mount
  createEffect(() => {
    context.registerPanel(
      panelId,
      local.defaultSize ?? 50,
      local.minSize ?? 0,
      local.maxSize ?? 100
    );
  });

  const size = context.getPanelSize(panelId);

  const style = (): JSX.CSSProperties => {
    const isHorizontal = context.direction() === "horizontal";
    return {
      [isHorizontal ? "width" : "height"]: `${size()}%`,
      "flex-shrink": 0,
      "flex-grow": 0,
    };
  };

  return (
    <div
      class={cn("ui-resizable-panel", local.class)}
      style={style()}
      data-panel-id={panelId}
      {...others}
    >
      {local.children}
    </div>
  );
};

// Handle
export interface ResizableHandleProps extends JSX.HTMLAttributes<HTMLDivElement> {
  withHandle?: boolean;
}

let handleCounter = 0;

export const ResizableHandle: Component<ResizableHandleProps> = (props) => {
  const [local, others] = splitProps(props, ["class", "withHandle"]);

  const context = useResizable();
  const handleIndex = handleCounter++;

  const handlePointerDown = (e: PointerEvent) => {
    context.startResize(handleIndex, e);
  };

  // Reset counter when component unmounts (for HMR)
  onCleanup(() => {
    handleCounter = 0;
  });

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
