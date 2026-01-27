import { Component, Show } from "solid-js";
import { appActions, useAppStore } from "../../core/store/appStore";
import { Search, ChevronLeft, ChevronRight, Plus, LoaderCircle } from "lucide-solid";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import "./primary-header.css";

export const PrimaryHeader: Component = () => {
  const { searchQuery, progress } = useAppStore();

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
                 value={searchQuery()}
                 onInput={(e) => appActions.setSearch(e.currentTarget.value)}
                 leftIcon={<Search size={14} />}
               />
           </div>
       </div>

       {/* Actions / Status */}
       <div class="header-actions">
            <Show when={progress()}>
                <div class="indexing-status">
                    <LoaderCircle size={12} class="spin" />
                    <span>Indexing {progress()?.processed} / {progress()?.total}</span>
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
