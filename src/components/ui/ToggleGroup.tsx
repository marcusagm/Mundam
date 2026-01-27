import { 
  Component, 
  JSX, 
  splitProps, 
  createContext, 
  useContext, 
  createMemo,
  Accessor 
} from "solid-js";
import { cn } from "../../lib/utils";
import { createControllableSignal } from "../../lib/primitives";
import "./toggle-group.css";

type ToggleGroupType = "single" | "multiple";

// Context
interface ToggleGroupContextValue {
  type: ToggleGroupType;
  value: Accessor<string | string[]>;
  onItemClick: (itemValue: string) => void;
  disabled: boolean;
}

const ToggleGroupContext = createContext<ToggleGroupContextValue>();

const useToggleGroup = () => {
  const context = useContext(ToggleGroupContext);
  if (!context) {
    throw new Error("ToggleGroupItem must be used within a ToggleGroup");
  }
  return context;
};

// ToggleGroup Root
export interface ToggleGroupSingleProps {
  type: "single";
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
}

export interface ToggleGroupMultipleProps {
  type: "multiple";
  value?: string[];
  defaultValue?: string[];
  onValueChange?: (value: string[]) => void;
}

export type ToggleGroupProps = (ToggleGroupSingleProps | ToggleGroupMultipleProps) & {
  disabled?: boolean;
  orientation?: "horizontal" | "vertical";
  class?: string;
  children: JSX.Element;
};

/**
 * ToggleGroup for selecting one or multiple options.
 * 
 * @example
 * // Single selection
 * <ToggleGroup type="single" defaultValue="center">
 *   <ToggleGroupItem value="left"><AlignLeft /></ToggleGroupItem>
 *   <ToggleGroupItem value="center"><AlignCenter /></ToggleGroupItem>
 *   <ToggleGroupItem value="right"><AlignRight /></ToggleGroupItem>
 * </ToggleGroup>
 * 
 * // Multiple selection
 * <ToggleGroup type="multiple" defaultValue={["bold"]}>
 *   <ToggleGroupItem value="bold"><Bold /></ToggleGroupItem>
 *   <ToggleGroupItem value="italic"><Italic /></ToggleGroupItem>
 * </ToggleGroup>
 */
export const ToggleGroup: Component<ToggleGroupProps> = (props) => {
  const [local] = splitProps(props, [
    "class",
    "type",
    "value",
    "defaultValue",
    "onValueChange",
    "disabled",
    "orientation",
    "children",
  ]);

  const isSingle = () => local.type === "single";

  // Single mode
  const singleState = createControllableSignal<string>({
    value: isSingle() ? () => (local as ToggleGroupSingleProps).value : undefined,
    defaultValue: isSingle() ? ((local as ToggleGroupSingleProps).defaultValue ?? "") : "",
    onChange: isSingle() ? (local as ToggleGroupSingleProps).onValueChange : undefined,
  });

  // Multiple mode
  const multipleState = createControllableSignal<string[]>({
    value: !isSingle() ? () => (local as ToggleGroupMultipleProps).value : undefined,
    defaultValue: !isSingle() ? ((local as ToggleGroupMultipleProps).defaultValue ?? []) : [],
    onChange: !isSingle() ? (local as ToggleGroupMultipleProps).onValueChange : undefined,
  });

  const value = createMemo(() => {
    if (isSingle()) {
      return singleState.value();
    }
    return multipleState.value();
  });

  const onItemClick = (itemValue: string) => {
    if (local.disabled) return;

    if (isSingle()) {
      // Single: toggle or set
      const current = singleState.value();
      singleState.setValue(current === itemValue ? "" : itemValue);
    } else {
      // Multiple: add or remove
      const current = multipleState.value();
      if (current.includes(itemValue)) {
        multipleState.setValue(current.filter((v: string) => v !== itemValue));
      } else {
        multipleState.setValue([...current, itemValue]);
      }
    }
  };

  const contextValue: ToggleGroupContextValue = {
    type: local.type,
    value,
    onItemClick,
    disabled: local.disabled ?? false,
  };

  return (
    <ToggleGroupContext.Provider value={contextValue}>
      <div
        class={cn(
          "ui-toggle-group",
          `ui-toggle-group-${local.orientation || "horizontal"}`,
          local.class
        )}
        role="group"
        aria-disabled={local.disabled}
      >
        {local.children}
      </div>
    </ToggleGroupContext.Provider>
  );
};

// ToggleGroup Item
export interface ToggleGroupItemProps extends Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "value"> {
  value: string;
  children: JSX.Element;
}

export const ToggleGroupItem: Component<ToggleGroupItemProps> = (props) => {
  const [local, others] = splitProps(props, [
    "class",
    "value",
    "disabled",
    "children",
  ]);

  const context = useToggleGroup();

  const isPressed = createMemo(() => {
    const currentValue = context.value();
    if (context.type === "single") {
      return currentValue === local.value;
    }
    return (currentValue as string[]).includes(local.value);
  });

  const isDisabled = () => local.disabled || context.disabled;

  const handleClick = () => {
    if (isDisabled()) return;
    context.onItemClick(local.value);
  };

  return (
    <button
      type="button"
      class={cn(
        "ui-toggle-group-item",
        isPressed() && "ui-toggle-group-item-pressed",
        local.class
      )}
      aria-pressed={isPressed()}
      disabled={isDisabled()}
      onClick={handleClick}
      {...others}
    >
      {local.children}
    </button>
  );
};
