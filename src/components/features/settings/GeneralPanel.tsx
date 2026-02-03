import { Component, createSignal } from 'solid-js';
import { Button } from '../../ui/Button';
import { toast } from '../../ui/Sonner';
import { tauriService } from '../../../core/tauri/services';

export const GeneralPanel: Component = () => {
    const [optimizing, setOptimizing] = createSignal(false);

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

    return (
    <div class="settings-panel-content">
      <h2 class="settings-panel-title">General</h2>
      
      <div class="settings-section">
        <h3 class="settings-section-title">Library Maintenance</h3>
        <p class="settings-section-description">
            Optimize the database to improve performance and reduce file size (VACUUM + ANALYZE).
        </p>
        <div style={{ "margin-top": "16px" }}>
            <Button onClick={handleOptimize} loading={optimizing()}>
                Optimize Library
            </Button>
        </div>
      </div>
    </div>
  );
};
