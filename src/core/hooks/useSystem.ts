import {
    loading,
    progress,
    indexingOutcome,
    thumbnailProgress,
    rootPath,
    isSettingsOpen,
    isDesignSystemOpen,
    systemActions
} from '../store/systemStore';

/**
 * Hook providing access to the overall system state like loading status and root path.
 *
 * @returns {Object} System state accessors and basic initialization methods.
 */
export const useSystem = () => {
    return {
        /** Overall application loading state flag */
        loading,
        /** Global progress tracking mapping */
        progress,
        indexingOutcome,
        /** Thumbnail generation progress */
        thumbnailProgress,
        /** Scanned library root path directory */
        rootPath,
        /** Modal state for settings */
        isSettingsOpen,
        /** Modal state for design system */
        isDesignSystemOpen,

        /** Core actions startup call */
        initialize: systemActions.initialize,
        /** Triggers a root configuration path change */
        setRootLocation: systemActions.setRootLocation,
        /** Send progress events */
        updateProgress: systemActions.updateProgress,
        /** Discards cached progress notifications */
        clearProgress: systemActions.clearProgress,

        /** Toggle settings modal */
        openSettings: systemActions.openSettings,
        /** Toggle design system modal */
        openDesignSystem: systemActions.openDesignSystem,
        /** Database maintenance task */
        runDbMaintenance: systemActions.runDbMaintenance,
        /** Cache cleanup */
        cleanupCache: systemActions.cleanupCache,
        /** Total cache clearing */
        clearCache: systemActions.clearCache
    };
};
