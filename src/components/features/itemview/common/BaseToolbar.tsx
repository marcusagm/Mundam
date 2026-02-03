import { ParentComponent } from "solid-js";
import { X, ChevronLeft, ChevronRight } from "lucide-solid";
import { useViewport, useLibrary } from "../../../../core/hooks";
import { Button } from "../../../ui/Button";
import { ButtonGroup } from "../../../ui/ButtonGroup";
import { Tooltip } from "../../../ui/Tooltip";
import { ShortcutHint } from "./ToolbarUtils";
import "../item-view-toolbar.css";
// import { useItemViewContext } from "../ItemViewContext";

export const BaseToolbar: ParentComponent = (props) => {
    const viewport = useViewport();
    const lib = useLibrary();
    // We might need context later, but for now just viewport/lib for nav
    // const { zoom } = useItemViewContext(); // Ensure context is available if needed

    const navigate = (direction: number) => {
        const items = lib.items;
        const currentId = viewport.activeItemId();
        const currentIndex = items.findIndex(i => i.id.toString() === currentId);
        
        if (currentIndex !== -1) {
            const nextIndex = (currentIndex + direction + items.length) % items.length;
            viewport.openItem(items[nextIndex].id.toString());
        }
    };

    return (
        <div class="item-view-toolbar">
            <div class="toolbar-group">
                <Tooltip position="bottom" content={<ShortcutHint name="Close Viewer" />}>
                    <Button variant="ghost" size="icon" onClick={() => viewport.closeItem()}>
                        <X size={18} />
                    </Button>
                </Tooltip>
            </div>

            {/* Renderer Specific Controls */}
            {props.children}

            <div class="toolbar-group ml-auto">
                <ButtonGroup>
                    <Tooltip position="bottom" content={<ShortcutHint name="Previous Item" />}>
                        <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
                            <ChevronLeft size={18} />
                        </Button>
                    </Tooltip>
                    <Tooltip position="bottom" content={<ShortcutHint name="Next Item" />}>
                        <Button variant="ghost" size="icon" onClick={() => navigate(1)}>
                            <ChevronRight size={18} />
                        </Button>
                    </Tooltip>
                </ButtonGroup>
            </div>
        </div>
    );
};
