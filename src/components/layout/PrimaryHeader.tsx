import { Component, Show } from "solid-js";
import { useFilters, useSystem } from "../../core/hooks";
import { Search, ChevronLeft, ChevronRight, Plus } from "lucide-solid";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { Loader } from "../ui/Loader";
import "./primary-header.css";

export const PrimaryHeader: Component = () => {
  const filters = useFilters();
  const system = useSystem();

  return (
    <div class="primary-header">
       {/* Navigation */}
       <div class="header-nav">
         <Button variant="ghost" size="icon" disabled>
            <ChevronLeft size={16} />
         </Button>
         <Button variant="ghost" size="icon" disabled>
            <ChevronRight size={16} />
         </Button>
       </div>

       {/* OmniSearch */}
       <div class="header-search">
           <div class="header-search-wrapper">
               <Input 
                 placeholder="Search references (Ctrl+K)" 
                 value={filters.searchQuery}
                 onInput={(e) => filters.setSearch(e.currentTarget.value)}
                 leftIcon={<Search size={14} />}
               />
           </div>
       </div>

       {/* Actions / Status */}
       <div class="header-actions">
            <Show when={system.progress()}>
                <div class="indexing-status">
                    <Loader size="sm" />
                    <span>Indexing {system.progress()?.processed} / {system.progress()?.total}</span>
                </div>
            </Show>
            <Button variant="primary" size="sm">
                <Plus size={14} />
                <span>Add</span>
            </Button>
       </div>
    </div>
  );
};
