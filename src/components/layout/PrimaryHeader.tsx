import { Component, Show } from "solid-js";
import { useSystem } from "../../core/hooks";
import { Settings } from "lucide-solid";
import { Button } from "../ui/Button";
import { Loader } from "../ui/Loader";
import "./primary-header.css";

export const PrimaryHeader: Component = () => {
  const system = useSystem();

  return (
    <div class="primary-header">
       {/* Actions / Status */}
       <div class="header-actions">
            <Show when={system.progress()}>
                <div class="indexing-status">
                    <Loader size="sm" />
                    <span>Indexing {system.progress()?.processed} / {system.progress()?.total}</span>
                </div>
            </Show>
            <Button variant="ghost" size="icon">
                <Settings />
            </Button>
       </div>
    </div>
  );
};
