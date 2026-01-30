import { Component } from "solid-js";
import { 
    ArrowLeft, 
    ArrowRight, 
    LayoutGrid, 
    AlignCenterVertical, 
    AlignCenterHorizontal, 
    List, 
    SortAsc, 
    SortDesc,
    ChevronDown
} from "lucide-solid";
import { useFilters, useViewport } from "../../../core/hooks";
import { Button } from "../../ui/Button";
import { SearchToolbar } from "../search/SearchToolbar";
import { DropdownMenu } from "../../ui/DropdownMenu";
import { ToggleGroup, ToggleGroupItem } from "../../ui/ToggleGroup";
import { Slider } from "../../ui/Slider";
import "./list-view-toolbar.css";

export const ListViewToolbar: Component = () => {
    const filters = useFilters();
    const viewport = useViewport();

    return (
        <div class="list-view-toolbar">
            {/* History Navigation */}
            <div class="toolbar-group">
                <Button 
                    variant="ghost" 
                    size="icon" 
                    onClick={() => viewport.goBack()} 
                    disabled={!viewport.canGoBack()}
                    title="Back"
                >
                    <ArrowLeft size={18} />
                </Button>
                <Button 
                    variant="ghost" 
                    size="icon" 
                    onClick={() => viewport.goForward()} 
                    disabled={!viewport.canGoForward()}
                    title="Forward"
                >
                    <ArrowRight size={18} />
                </Button>
            </div>

            {/* Search Bar */}
            <div class="toolbar-search">
                <SearchToolbar />
            </div>

            {/* Sort & View Controls */}
            <div class="toolbar-group">
                {/* Sort Dropdown */}
                <DropdownMenu
                    trigger={
                        <Button variant="ghost" class="sort-dropdown-trigger">
                            <span>Sort: {
                                {
                                    modified_at: "Modification",
                                    added_at: "Addition",
                                    created_at: "Creation",
                                    filename: "Title",
                                    format: "Type",
                                    size: "Size",
                                    rating: "Rating"
                                }[filters.sortBy] || "Date"
                            }</span>
                            <ChevronDown size={14} />
                        </Button>
                    }
                    items={[
                        { type: "item", label: "Modification Date", action: () => filters.setSortBy("modified_at") },
                        { type: "item", label: "Addition Date", action: () => filters.setSortBy("added_at") },
                        { type: "item", label: "Creation Date", action: () => filters.setSortBy("created_at") },
                        { type: "item", label: "Title", action: () => filters.setSortBy("filename") },
                        { type: "item", label: "File Type", action: () => filters.setSortBy("format") },
                        { type: "item", label: "File Size", action: () => filters.setSortBy("size") },
                        { type: "item", label: "Rating", action: () => filters.setSortBy("rating") },
                    ]}
                />

                {/* Sort Order Toggle */}
                <ToggleGroup 
                    type="single" 
                    value={filters.sortOrder} 
                    onValueChange={(val) => filters.setSortOrder(val as "asc" | "desc")}
                >
                    <ToggleGroupItem value="asc" title="Ascending">
                        <SortAsc size={14} />
                    </ToggleGroupItem>
                    <ToggleGroupItem value="desc" title="Descending">
                        <SortDesc size={14} />
                    </ToggleGroupItem>
                </ToggleGroup>

                <div class="toolbar-separator" />

                {/* Layout Dropdown */}
                <DropdownMenu
                    trigger={
                        <Button variant="ghost" size="icon" title="View Layout">
                            <LayoutGrid size={18} />
                        </Button>
                    }
                    items={[
                        { type: "item", label: "Masonry Vertical", icon: AlignCenterVertical as any, action: () => filters.setLayout("masonry-v") },
                        { type: "item", label: "Masonry Horizontal", icon: AlignCenterHorizontal as any, action: () => filters.setLayout("masonry-h") },
                        { type: "item", label: "Grid", icon: LayoutGrid as any, action: () => filters.setLayout("grid") },
                        { type: "item", label: "List", icon: List as any, action: () => filters.setLayout("list") },
                    ]}
                />
                
                {/* O Slider de tamanho de thumbnail precisa de um lugar melhor se o DropdownMenu for simplificado assim, 
                    ou podemos adicionar um item customizado se o Dropdown suportar. Por enquanto, vamos deixar o Slider fora se necessário ou ajustar o Dropdown.
                    Dado que o DropdownMenu atual não suporta itens customizados facilmente via props.items, 
                    vamos manter o Slider como um elemento separado por enquanto para não quebrar a UI.
                */}
                <div class="toolbar-group" style={{ 
                    width: "120px", 
                    "margin-left": "8px",
                    "display": "flex",
                    "align-items": "center",
                    "gap": "8px"
                }}>
                    <Slider 
                        value={filters.thumbSize || 200} 
                        min={100} 
                        max={500} 
                        step={filters.layout === 'grid' ? 20 : 10}
                        onValueChange={(val) => filters.setThumbSize(val)}
                        title="Thumbnail Size"
                    />
                </div>
            </div>
        </div>
    );
};
