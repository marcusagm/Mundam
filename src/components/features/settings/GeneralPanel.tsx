import { Component, createSignal } from 'solid-js';
import { Button } from '../../ui/Button';
import { toast } from '../../ui/Sonner';
import { SectionGroup } from '../../ui/SectionGroup';
import { Select } from '../../ui/Select';
import { tauriService } from '../../../core/tauri/services';
import './general-panel.css';

export const GeneralPanel: Component = () => {
    const [optimizing, setOptimizing] = createSignal(false);
    const [threads, setThreads] = createSignal<string>("2");

    // Load initial settings
    import("solid-js").then(({ onMount }) => {
        onMount(async () => {
            const val = await tauriService.getSetting("thumbnail_threads");
            if (val) setThreads(String(val));
        });
    });

    const handleOptimize = async () => {
        setOptimizing(true);
        toast.info("Starting database optimization...");
        try {
            await tauriService.runDbMaintenance();
            toast.success("Database optimization complete.");
        } catch (e) {
            toast.error("Failed to optimize database.");
            console.error(e);
        } finally {
            setOptimizing(false);
        }
    };

    const handleThreadChange = async (val: string) => {
        setThreads(val);
        try {
            await tauriService.setSetting("thumbnail_threads", parseInt(val));
            toast.success("Settings saved. Please restart the app for changes to take effect.");
        } catch (e) {
            toast.error("Failed to save settings.");
        }
    };

    const threadOptions = [
        { value: "1", label: "1 (Low CPU)" },
        { value: "2", label: "2 (Balanced)" },
        { value: "4", label: "4 (High Performance)" },
        { value: "8", label: "8 (Extreme)" },
    ];

    return (
    <div class="settings-panel-content general-panel">
      <h2 class="settings-panel-title">General</h2>
      
      <SectionGroup 
        title="Performance" 
        description="Configure background processing power. Higher values use more CPU but generate thumbnails faster."
      >
        <div class="general-setting-row">
            <span class="setting-label">Thumbnail Threads:</span>
            <div style={{ width: "200px" }}>
                <Select
                    options={threadOptions}
                    value={threads()}
                    onValueChange={handleThreadChange}
                    placeholder="Select threads"
                />
            </div>
        </div>
        <p class="setting-note">
            * Requires restart to apply.
        </p>
      </SectionGroup>

      <SectionGroup 
        title="Library Maintenance" 
        description="Optimize the database to improve performance and reduce file size (VACUUM + ANALYZE)."
      >
        <div class="setting-action-row">
            <Button onClick={handleOptimize} loading={optimizing()}>
                Optimize Library
            </Button>
        </div>
      </SectionGroup>
    </div>
  );
};
