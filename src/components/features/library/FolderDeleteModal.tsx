import { Component, createSignal } from "solid-js";
import { ConfirmModal } from "../../ui/Modal";
import { invoke } from "@tauri-apps/api/core";
import "./folder-delete-modal.css";

interface Location {
    id: number;
    path: string;
    name: string;
}

interface FolderDeleteModalProps {
    isOpen: boolean;
    onClose: () => void;
    location: Location | null;
    onDeleted: () => void;
}

export const FolderDeleteModal: Component<FolderDeleteModalProps> = (props) => {
    const [isDeleting, setIsDeleting] = createSignal(false);

    const handleConfirm = async () => {
        const loc = props.location;
        if (!loc) return;

        setIsDeleting(true);
        try {
            await invoke("remove_location", { locationId: loc.id });
            props.onDeleted();
        } catch (err) {
            console.error("Failed to delete folder:", err);
        } finally {
            setIsDeleting(false);
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
            confirmText={isDeleting() ? "Removing..." : "Remove"}
            message=""
        >
            <div class="folder-delete-modal-content">
                <p>
                    Are you sure you want to remove <strong>"{props.location?.name}"</strong> from the library?
                </p>
                <p class="folder-delete-path">
                    {props.location?.path}
                </p>
                <p class="folder-delete-warning">
                    This will remove all images from this folder from the library and delete their thumbnails.
                    The original files will <strong>not</strong> be deleted.
                </p>
            </div>
        </ConfirmModal>
    );
};
