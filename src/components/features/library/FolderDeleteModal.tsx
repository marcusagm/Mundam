import { Component } from "solid-js";
import { ConfirmModal } from "../../ui/Modal";
import { invoke } from "@tauri-apps/api/core";
import { useMetadata, useNotification } from "../../../core/hooks";
import "./folder-delete-modal.css";

interface FolderDeleteModalProps {
    isOpen: boolean;
    onClose: () => void;
    folderId: number | null;
    folderName: string;
}

export const FolderDeleteModal: Component<FolderDeleteModalProps> = (props) => {
    const { loadLocations, loadStats } = useMetadata();
    const notification = useNotification();

    const handleConfirm = async () => {
        if (props.folderId === null) return;
        
        try {
            await invoke("remove_location", { locationId: props.folderId });
            await loadLocations();
            await loadStats();
            notification.success("Folder Removed", `Stopped monitoring "${props.folderName}"`);
        } catch (err) {
            console.error("Failed to remove folder:", err);
            notification.error("Failed to Remove Folder");
        } finally {
            props.onClose();
        }
    };

    return (
        <ConfirmModal 
            isOpen={props.isOpen}
            onClose={props.onClose}
            onConfirm={handleConfirm}
            title="Remove Folder"
            kind="danger"
            confirmText="Remove"
            message=""
        >
            <div class="folder-delete-modal-content">
                <p>
                    Are you sure you want to remove <strong>"{props.folderName}"</strong> from the library?
                </p>
                <p class="folder-delete-warning">
                    This will remove all images from this folder from the library and delete their thumbnails.
                    The original files will <strong>not</strong> be deleted.
                </p>
            </div>
        </ConfirmModal>
    );
};
