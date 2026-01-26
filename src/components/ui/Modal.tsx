
import { Component, JSX, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { Button } from "./Button";
import { Input } from "./Input";

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: JSX.Element;
  footer?: JSX.Element;
}

export const Modal: Component<ModalProps> = (props) => {
    return (
        <Show when={props.isOpen}>
            <Portal>
                 <div style={{
                     position: "fixed",
                     inset: 0,
                     "z-index": 99999,
                     background: "rgba(0,0,0,0.5)",
                     display: "flex",
                     "align-items": "center",
                     "justify-content": "center",
                     "backdrop-filter": "blur(2px)"
                 }} onClick={props.onClose}>
                     <div style={{
                         background: "var(--bg-surface)",
                         border: "1px solid var(--border-color)",
                         "border-radius": "var(--radius-lg)",
                         padding: "16px",
                         "min-width": "300px",
                         "box-shadow": "var(--shadow-md)"
                     }} onClick={(e) => e.stopPropagation()}>
                         <div style={{ "margin-bottom": "12px", "font-weight": "600", "font-size": "14px" }}>
                             {props.title}
                         </div>
                         <div style={{ "margin-bottom": "16px" }}>
                             {props.children}
                         </div>
                         <Show when={props.footer}>
                             <div style={{ display: "flex", "justify-content": "flex-end", gap: "8px" }}>
                                 {props.footer}
                             </div>
                         </Show>
                     </div>
                 </div>
            </Portal>
        </Show>
    );
};

export const PromptModal: Component<{
    isOpen: boolean,
    onClose: () => void,
    onConfirm: (val: string) => void,
    title: string,
    initialValue?: string,
    placeholder?: string
}> = (props) => {
    let inputRef: HTMLInputElement | undefined;

    const handleSubmit = (e: Event) => {
        e.preventDefault();
        if (inputRef && inputRef.value) {
            props.onConfirm(inputRef.value);
            props.onClose();
        }
    }

    return (
        <Modal 
            isOpen={props.isOpen} 
            onClose={props.onClose} 
            title={props.title}
        >
            <form onSubmit={handleSubmit}>
                <Input 
                    ref={inputRef}
                    value={props.initialValue || ""} 
                    placeholder={props.placeholder}
                    autofocus
                />
                <div style={{ display: "flex", "justify-content": "flex-end", gap: "8px", "margin-top": "16px" }}>
                    <Button type="button" variant="ghost" onClick={props.onClose}>Cancel</Button>
                    <Button type="submit" variant="primary">Confirm</Button>
                </div>
            </form>
        </Modal>
    );
};

export const ConfirmModal: Component<{
    isOpen: boolean;
    onClose: () => void;
    onConfirm: () => void;
    title: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    kind?: "danger" | "warning" | "info";
}> = (props) => {
    return (
        <Modal
            isOpen={props.isOpen}
            onClose={props.onClose}
            title={props.title}
        >
            <div style={{ "margin-bottom": "24px", "font-size": "13px", "line-height": "1.5" }}>
                {props.message}
            </div>
            <div style={{ display: "flex", "justify-content": "flex-end", gap: "8px" }}>
                <Button variant="ghost" onClick={props.onClose}>
                    {props.cancelText || "Cancel"}
                </Button>
                <Button 
                    variant={props.kind === "danger" ? "destructive" : "primary"} 
                    onClick={() => {
                        props.onConfirm();
                        props.onClose();
                    }}
                >
                    {props.confirmText || "Confirm"}
                </Button>
            </div>
        </Modal>
    );
};

