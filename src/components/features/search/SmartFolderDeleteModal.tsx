import { Component } from "solid-js";
import { ConfirmModal } from "../../ui/Modal";
import { SmartFolder } from "../../../core/store/metadataStore";
import { useMetadata, useNotification } from "../../../core/hooks";

interface SmartFolderDeleteModalProps {
    isOpen: boolean;
    onClose: () => void;
    folder: SmartFolder | null;
}

export const SmartFolderDeleteModal: Component<SmartFolderDeleteModalProps> = (props) => {
    const metadata = useMetadata();
    const notification = useNotification();

    const handleConfirm = async () => {
        if (!props.folder) return;

        const folderName = props.folder.name;
        try {
            await metadata.deleteSmartFolder(props.folder.id);
            notification.success("Smart Folder Deleted", `Removed "${folderName}"`);
        } catch (err) {
            console.error("Delete failed:", err);
            notification.error("Failed to Delete Smart Folder");
        } finally {
            props.onClose();
        }
    };

    return (
        <ConfirmModal 
            isOpen={props.isOpen}
            onClose={props.onClose}
            onConfirm={handleConfirm}
            title="Delete Smart Folder"
            kind="danger"
            confirmText="Delete"
        >
            <div class="delete-confirmation-content">
                <p>
                    Are you sure you want to delete the smart folder <strong>"{props.folder?.name}"</strong>?
                </p>
                <p class="delete-warning">
                    This will only remove the saved search. Your images and actual folders will not be affected.
                </p>
            </div>
        </ConfirmModal>
    );
};
