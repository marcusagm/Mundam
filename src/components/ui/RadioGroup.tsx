import { 
  Component, 
  JSX, 
  splitProps, 
  createContext, 
  useContext, 
  createMemo 
} from "solid-js";
import { cn } from "../../lib/utils";
import { createControllableSignal } from "../../lib/primitives";
import { createId } from "../../lib/primitives/createId";
import "./radio-group.css";

// Context for RadioGroup
interface RadioGroupContextValue {
  name: string;
  value: () => string;
  onChange: (value: string) => void;
  disabled: boolean;
}

const RadioGroupContext = createContext<RadioGroupContextValue>();

const useRadioGroup = () => {
  const context = useContext(RadioGroupContext);
  if (!context) {
    throw new Error("RadioGroupItem must be used within a RadioGroup");
  }
  return context;
};

// RadioGroup Root
export interface RadioGroupProps {
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  name?: string;
  disabled?: boolean;
  orientation?: "horizontal" | "vertical";
  class?: string;
  children: JSX.Element;
}

/**
 * RadioGroup for selecting a single option from a list.
 * 
 * @example
 * <RadioGroup defaultValue="option1" onValueChange={console.log}>
 *   <RadioGroupItem value="option1" label="Option 1" />
 *   <RadioGroupItem value="option2" label="Option 2" />
 * </RadioGroup>
 */
export const RadioGroup: Component<RadioGroupProps> = (props) => {
  const [local] = splitProps(props, [
    "class",
    "value",
    "defaultValue",
    "onValueChange",
    "name",
    "disabled",
    "orientation",
    "children",
  ]);

  const name = createMemo(() => local.name || createId("radio-group"));

  const { value, setValue } = createControllableSignal({
    value: () => local.value,
    defaultValue: local.defaultValue ?? "",
    onChange: local.onValueChange,
  });

  const contextValue: RadioGroupContextValue = {
    name: name(),
    value,
    onChange: setValue,
    disabled: local.disabled ?? false,
  };

  return (
    <RadioGroupContext.Provider value={contextValue}>
      <div
        class={cn(
          "ui-radio-group",
          `ui-radio-group-${local.orientation || "vertical"}`,
          local.class
        )}
        role="radiogroup"
        aria-disabled={local.disabled}
      >
        {local.children}
      </div>
    </RadioGroupContext.Provider>
  );
};

// RadioGroup Item
export interface RadioGroupItemProps extends Omit<JSX.InputHTMLAttributes<HTMLInputElement>, "type" | "onChange"> {
  value: string;
  label?: string;
  description?: string;
}

export const RadioGroupItem: Component<RadioGroupItemProps> = (props) => {
  const [local, others] = splitProps(props, [
    "class",
    "value",
    "label",
    "description",
    "id",
    "disabled",
  ]);

  const context = useRadioGroup();
  const id = createMemo(() => local.id || createId("radio"));
  const descriptionId = createMemo(() => local.description ? `${id()}-desc` : undefined);

  const isSelected = createMemo(() => context.value() === local.value);
  const isDisabled = () => local.disabled || context.disabled;

  const handleClick = () => {
    if (isDisabled()) return;
    context.onChange(local.value);
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (isDisabled()) return;
    if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      context.onChange(local.value);
    }
  };

  return (
    <label
      class={cn(
        "ui-radio-wrapper",
        isDisabled() && "ui-radio-disabled",
        local.class
      )}
      for={id()}
    >
      <button
        type="button"
        role="radio"
        id={id()}
        class={cn("ui-radio", isSelected() && "ui-radio-checked")}
        aria-checked={isSelected()}
        aria-disabled={isDisabled()}
        aria-describedby={descriptionId()}
        disabled={isDisabled()}
        onClick={handleClick}
        onKeyDown={handleKeyDown}
      >
        <span class="ui-radio-indicator" />
      </button>

      {(local.label || local.description) && (
        <div class="ui-radio-content">
          {local.label && <span class="ui-radio-label">{local.label}</span>}
          {local.description && (
            <span id={descriptionId()} class="ui-radio-description">
              {local.description}
            </span>
          )}
        </div>
      )}

      {/* Hidden input for form submission */}
      <input
        type="radio"
        name={context.name}
        value={local.value}
        checked={isSelected()}
        disabled={isDisabled()}
        class="ui-radio-input"
        tabindex={-1}
        aria-hidden="true"
        {...others}
      />
    </label>
  );
};
