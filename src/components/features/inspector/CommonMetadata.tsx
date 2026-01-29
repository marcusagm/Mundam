import { Component, createSignal, createEffect } from "solid-js";
import { Info, FileText, Calendar, HardDrive } from "lucide-solid";
import { AccordionItem } from "../../ui/Accordion";
import { Input } from "../../ui/Input";
import { StarRating } from "./StarRating";
import { useLibrary } from "../../../core/hooks"; // Import hooks
import { type ImageItem } from "../../../types"; 
import "./inspector.css";

interface CommonMetadataProps {
    item: ImageItem | null;
}

const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
};

const formatDate = (dateStr: string) => {
    if (!dateStr) return "-";
    return new Date(dateStr).toLocaleDateString();
};

export const CommonMetadata: Component<CommonMetadataProps> = (props) => {
    // Local state for debounce
    const [notes, setNotes] = createSignal(props.item?.notes || "");
    const lib = useLibrary();

    // Sync notes when item changes
    createEffect(() => {
        setNotes(props.item?.notes || "");
    });

    const handleNotesChange = (val: string) => {
        setNotes(val);
        if (props.item) {
            lib.updateItemNotes(props.item.id, val);
        }
    };

    const handleRatingChange = (rating: number) => {
        if (props.item) {
            lib.updateItemRating(props.item.id, rating);
        }
    };

    return (
        <AccordionItem 
            value="common" 
            title="General Info" 
            defaultOpen 
            icon={<Info size={14} />}
        >
            <div class="inspector-field-group">
                <label class="inspector-label">Name</label>
                <Input value={props.item?.filename || ""} disabled />
            </div>

            <div class="inspector-field-group">
               <label class="inspector-label">Rating</label>
               <div class="inspector-rating-container">
                   <StarRating 
                        rating={props.item?.rating || 0} 
                        onChange={handleRatingChange} 
                   />
               </div>
            </div>

            <div class="inspector-grid">
                <div class="inspector-meta-item">
                    <span class="inspector-meta-label">Type</span>
                    <span class="inspector-meta-value">
                        <FileText size={10} />
                        {props.item?.filename.split('.').pop()?.toUpperCase()}
                    </span>
                </div>
                <div class="inspector-meta-item">
                     <span class="inspector-meta-label">Size</span>
                     <span class="inspector-meta-value">
                        <HardDrive size={10} />
                        {props.item ? formatBytes(props.item.size) : "-"}
                     </span>
                </div>
                 <div class="inspector-meta-item">
                     <span class="inspector-meta-label">Created</span>
                     <span class="inspector-meta-value">
                        <Calendar size={10} />
                        {props.item ? formatDate(props.item.created_at) : "-"}
                     </span>
                </div>
                 <div class="inspector-meta-item">
                     <span class="inspector-meta-label">Modified</span>
                     <span class="inspector-meta-value">
                        <Calendar size={10} />
                        {props.item ? formatDate(props.item.modified_at) : "-" }
                     </span>
                </div>
            </div>

            <div class="inspector-field-group" style="margin-top: 12px;">
                <label class="inspector-label">Notes</label>
                <textarea 
                    class="inspector-notes-input"
                    value={notes()}
                    onInput={(e) => handleNotesChange(e.currentTarget.value)}
                    placeholder="Add observations..."
                    rows={3}
                />
            </div>
        </AccordionItem>
    );
};
