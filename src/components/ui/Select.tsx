import {
  Component,
  JSX,
  splitProps,
  createSignal,
  createEffect,
  Show,
  For,
  createMemo,
} from "solid-js";
import { Portal } from "solid-js/web";
import { ChevronDown, Check, X } from "lucide-solid";
import { cn } from "../../lib/utils";
import { createControllableSignal, createClickOutside } from "../../lib/primitives";
import { createId } from "../../lib/primitives/createId";
import "./select.css";

export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface SelectProps extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "onChange"> {
  options: SelectOption[];
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  clearable?: boolean;
  searchable?: boolean;
  name?: string;
  error?: boolean;
  errorMessage?: string;
  leftIcon?: JSX.Element;
  rightIcon?: JSX.Element;
  size?: "sm" | "md" | "lg";
}

/**
 * Select component for choosing from a list of options.
 * Supports search, clear, and custom rendering.
 * 
 * @example
 * <Select
 *   options={[
 *     { value: "apple", label: "Apple" },
 *     { value: "banana", label: "Banana" },
 *   ]}
 *   placeholder="Select a fruit"
 *   onValueChange={(val) => console.log(val)}
 * />
 */
export const Select: Component<SelectProps> = (props) => {
  const [local, others] = splitProps(props, [
    "class",
    "options",
    "value",
    "defaultValue",
    "onValueChange",
    "placeholder",
    "disabled",
    "clearable",
    "searchable",
    "name",
    "error",
    "errorMessage",
    "id",
    "leftIcon",
    "rightIcon",
    "size",
  ]);

  const id = createMemo(() => local.id || createId("select"));

  const [isOpen, setIsOpen] = createSignal(false);
  const [searchQuery, setSearchQuery] = createSignal("");
  const [highlightedIndex, setHighlightedIndex] = createSignal(-1);
  const [position, setPosition] = createSignal({ top: 0, left: 0, width: 0 });

  let triggerRef: HTMLButtonElement | undefined;
  let contentRef: HTMLDivElement | undefined;
  let searchInputRef: HTMLInputElement | undefined;

  const { value, setValue } = createControllableSignal({
    value: () => local.value,
    defaultValue: local.defaultValue ?? "",
    onChange: local.onValueChange,
  });

  const selectedOption = createMemo(() =>
    local.options.find((opt) => opt.value === value())
  );

  const filteredOptions = createMemo(() => {
    if (!local.searchable || !searchQuery()) {
      return local.options;
    }
    const query = searchQuery().toLowerCase();
    return local.options.filter((opt) =>
      opt.label.toLowerCase().includes(query)
    );
  });

  const open = () => {
    if (local.disabled) return;
    setIsOpen(true);
    setSearchQuery("");
    setHighlightedIndex(-1);
  };

  const close = () => {
    setIsOpen(false);
    setSearchQuery("");
  };

  const selectOption = (option: SelectOption) => {
    if (option.disabled) return;
    setValue(option.value);
    close();
  };

  const clearValue = (e: Event) => {
    e.stopPropagation();
    setValue("");
  };

  // Calculate position
  createEffect(() => {
    if (isOpen() && triggerRef && contentRef) {
      const rect = triggerRef.getBoundingClientRect();
      const contentRect = contentRef.getBoundingClientRect();
      const viewport = { height: window.innerHeight };

      let top = rect.bottom + 4;
      
      // Check if dropdown would go off screen
      if (top + contentRect.height > viewport.height - 8) {
        top = rect.top - contentRect.height - 4;
      }

      setPosition({
        top,
        left: rect.left,
        width: rect.width,
      });
    }
  });

  // Focus search input when opened
  createEffect(() => {
    if (isOpen() && local.searchable && searchInputRef) {
      searchInputRef.focus();
    }
  });

  // Click outside
  createClickOutside(
    () => contentRef,
    () => {
      if (isOpen()) close();
    }
  );

  // Keyboard navigation
  const handleKeyDown = (e: KeyboardEvent) => {
    if (!isOpen()) {
      if (e.key === "Enter" || e.key === " " || e.key === "ArrowDown") {
        e.preventDefault();
        open();
      }
      return;
    }

    const options = filteredOptions();

    switch (e.key) {
      case "Escape":
        e.preventDefault();
        close();
        break;
      case "ArrowDown":
        e.preventDefault();
        setHighlightedIndex((i) => Math.min(i + 1, options.length - 1));
        break;
      case "ArrowUp":
        e.preventDefault();
        setHighlightedIndex((i) => Math.max(i - 1, 0));
        break;
      case "Enter":
        e.preventDefault();
        const highlighted = options[highlightedIndex()];
        if (highlighted && !highlighted.disabled) {
          selectOption(highlighted);
        }
        break;
    }
  };

  const size = () => local.size || "md";

  return (
    <div class={cn("ui-select", local.class)} {...others}>
      <button
        ref={triggerRef}
        type="button"
        id={id()}
        class={cn(
          "ui-select-trigger",
          `ui-select-trigger-${size()}`,
          isOpen() && "ui-select-trigger-open",
          local.disabled && "ui-select-trigger-disabled",
          local.error && "ui-select-trigger-error",
          !!local.leftIcon && "ui-select-has-left-icon",
          !!local.rightIcon && "ui-select-has-right-icon"
        )}
        role="combobox"
        aria-expanded={isOpen()}
        aria-haspopup="listbox"
        aria-disabled={local.disabled}
        disabled={local.disabled}
        onClick={() => (isOpen() ? close() : open())}
        onKeyDown={handleKeyDown}
      >
        <Show when={local.leftIcon}>
            <span class="ui-select-icon-left">{local.leftIcon}</span>
        </Show>

        <span class={cn("ui-select-value", !selectedOption() && "ui-select-placeholder")}>
          {selectedOption()?.label || local.placeholder || "Select..."}
        </span>

        <Show when={local.rightIcon}>
            <span class="ui-select-icon-right">{local.rightIcon}</span>
        </Show>

        <div class="ui-select-icons">
          <Show when={local.clearable && value()}>
            <span class="ui-select-clear" onClick={clearValue} aria-label="Clear">
              <X size={14} />
            </span>
          </Show>
          <ChevronDown
            size={16}
            class={cn("ui-select-chevron", isOpen() && "ui-select-chevron-open")}
          />
        </div>
      </button>

      <Show when={isOpen()}>
        <Portal>
          <div
            ref={contentRef}
            class="ui-select-content"
            style={{
              position: "fixed",
              top: `${position().top}px`,
              left: `${position().left}px`,
              width: `${position().width}px`,
              "z-index": 9999,
            }}
          >
            <Show when={local.searchable}>
              <div class="ui-select-search">
                <input
                  ref={searchInputRef}
                  type="text"
                  class="ui-select-search-input"
                  placeholder="Search..."
                  value={searchQuery()}
                  onInput={(e) => {
                    setSearchQuery(e.currentTarget.value);
                    setHighlightedIndex(0);
                  }}
                  onKeyDown={handleKeyDown}
                />
              </div>
            </Show>

            <div class="ui-select-options" role="listbox">
              <Show
                when={filteredOptions().length > 0}
                fallback={
                  <div class="ui-select-empty">No options found</div>
                }
              >
                <For each={filteredOptions()}>
                  {(option, index) => (
                    <div
                      class={cn(
                        "ui-select-option",
                        option.value === value() && "ui-select-option-selected",
                        option.disabled && "ui-select-option-disabled",
                        highlightedIndex() === index() && "ui-select-option-highlighted"
                      )}
                      role="option"
                      aria-selected={option.value === value()}
                      aria-disabled={option.disabled}
                      onClick={() => selectOption(option)}
                      onMouseEnter={() => setHighlightedIndex(index())}
                    >
                      <span class="ui-select-option-label">{option.label}</span>
                      <Show when={option.value === value()}>
                        <Check size={14} class="ui-select-check" />
                      </Show>
                    </div>
                  )}
                </For>
              </Show>
            </div>
          </div>
        </Portal>
      </Show>

      <Show when={local.error && local.errorMessage}>
        <span class="ui-select-error-message" role="alert">
          {local.errorMessage}
        </span>
      </Show>

      {/* Hidden input for form submission */}
      <input
        type="hidden"
        name={local.name}
        value={value()}
      />
    </div>
  );
};
