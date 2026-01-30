import { metadataState, metadataActions } from "../store/metadataStore";

export const useMetadata = () => {
  return {
    // State
    get tags() { return metadataState.tags; },
    get locations() { return metadataState.locations; },
    get stats() { return metadataState.libraryStats; },
    get smartFolders() { return metadataState.smartFolders; },
    get tagUpdateVersion() { return metadataState.tagUpdateVersion; },
    
    // Actions
    loadTags: metadataActions.loadTags,
    loadLocations: metadataActions.loadLocations,
    loadStats: metadataActions.loadStats,
    loadSmartFolders: metadataActions.loadSmartFolders,
    saveSmartFolder: metadataActions.saveSmartFolder,
    deleteSmartFolder: metadataActions.deleteSmartFolder,
    refreshAll: metadataActions.refreshAll,
    notifyTagUpdate: metadataActions.notifyTagUpdate
  };
};
