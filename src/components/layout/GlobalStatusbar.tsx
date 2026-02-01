import { Component } from "solid-js";
import { StatusCounts } from "../features/statusbar/StatusCounts";
import { StatusMessages } from "../features/statusbar/StatusMessages";
import { StatusSystem } from "../features/statusbar/StatusSystem";
import { PanelLeftClose, PanelLeftOpen, PanelRightClose, PanelRightOpen } from "lucide-solid";
import { Button } from "../ui/Button";
import { useAppShell } from "../../layouts/AppShell";
import "./global-statusbar.css";

export const GlobalStatusbar: Component = () => {
  const shell = useAppShell();

  return (
    <div class="global-statusbar">
       <div class="statusbar-left">
          <Button 
            variant="ghost" 
            size="icon-sm" 
            class="status-btn"
            onClick={() => shell?.toggleSidebar()}
            title={shell?.isSidebarOpen() ? "Close Sidebar" : "Open Sidebar"}
          >
             {shell?.isSidebarOpen() ? <PanelLeftClose size={14} /> : <PanelLeftOpen size={14} />}
          </Button>
          
          <div style={{ "margin-left": "8px", display: "flex" }}>
            <StatusCounts />
          </div>
       </div>
       
       <div class="statusbar-center">
          <StatusMessages />
       </div>
       
       <div class="statusbar-right">
          <StatusSystem />
          
          <div style={{ "margin-left": "8px" }}>
            <Button 
                variant="ghost" 
                size="icon-sm" 
                class="status-btn"
                onClick={() => shell?.toggleInspector()}
                title={shell?.isInspectorOpen() ? "Close Inspector" : "Open Inspector"}
            >
                {shell?.isInspectorOpen() ? <PanelRightClose size={14} /> : <PanelRightOpen size={14} />}
            </Button>
          </div>
       </div>
    </div>
  );
};
