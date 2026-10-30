/** Configuration shared by virtualized thumbnail consumers. */
export interface ThumbnailProperties {
    /** Unique asset identity. */
    id: string;
    /** Original source path; original media is never a thumbnail fallback. */
    src: string;
    /** Last known generated thumbnail. */
    thumbnail: string | null;
    /** Alternative image description and filename. */
    alt: string;
    /** Original media width. */
    width?: number | null;
    /** Original media height. */
    height?: number | null;
    /** Media family used by the file icon. */
    mediaType?: string;
    /** Legacy lifecycle state, used only for unavailable-source fallback. */
    state?: string;
}
