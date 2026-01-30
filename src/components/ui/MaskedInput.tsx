import { Component, splitProps } from "solid-js";
import { Input, InputProps } from "./Input";

export interface MaskedInputProps extends Omit<InputProps, "onInput"> {
  mask: string; // e.g. "99/99/9999"
  onInput?: (value: string) => void;
}

/**
 * A simple masked input component.
 * Supports '9' for digits.
 */
export const MaskedInput: Component<MaskedInputProps> = (props) => {
  const [local, others] = splitProps(props, ["mask", "onInput", "value"]);

  const applyMask = (val: string) => {
    let result = "";
    let valIndex = 0;
    const cleanVal = val.replace(/\D/g, "");

    for (let i = 0; i < local.mask.length && valIndex < cleanVal.length; i++) {
      const maskChar = local.mask[i];
      if (maskChar === "9") {
        result += cleanVal[valIndex];
        valIndex++;
      } else {
        result += maskChar;
      }
    }
    return result;
  };

  const handleInput = (e: InputEvent & { currentTarget: HTMLInputElement }) => {
    const masked = applyMask(e.currentTarget.value);
    e.currentTarget.value = masked;
    local.onInput?.(masked);
  };

  return (
    <Input
      {...others}
      value={local.value}
      onInput={handleInput}
    />
  );
};
