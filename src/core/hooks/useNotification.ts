import { toast } from "../../components/ui/Sonner";

export const useNotification = () => {
    return {
        success: (title: string, description?: string, action?: { label: string; onClick: () => void }) => {
            toast.success(title, { description, action });
        },
        error: (title: string, description?: string) => {
            toast.error(title, { description });
        },
        info: (title: string, description?: string) => {
            toast.info(title, { description });
        },
        warning: (title: string, description?: string) => {
            toast.warning(title, { description });
        },
        dismiss: (id: string) => {
            toast.dismiss(id);
        }
    };
};
