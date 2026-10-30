/** Ordered milestones exposed by the backend startup service. */
export type StartupStage =
    | 'preparing'
    | 'database'
    | 'media'
    | 'library'
    | 'backendReady'
    | 'appearance'
    | 'content'
    | 'interface'
    | 'ready';

/** Retained state, available before library services exist. */
export interface StartupSnapshot {
    /** Monotonic sequence used to reject stale event/query responses. */
    revision: number;
    /** Most recent boot milestone. */
    stage: StartupStage;
    /** Indicates that library-dependent IPC services have been registered. */
    backendReady: boolean;
    /** Terminal failure; later milestones cannot recover a partial boot. */
    failed: boolean;
    /** Safe diagnostic code; internal error details stay in backend tracing. */
    errorCode: string | null;
}

/** Presentation messages avoid exposing infrastructure details. */
export const STARTUP_MESSAGES: Record<StartupStage, string> = {
    preparing: 'Preparing your workspace',
    database: 'Opening your library',
    media: 'Preparing media services',
    library: 'Connecting your folders',
    backendReady: 'Loading your workspace',
    appearance: 'Applying your appearance',
    content: 'Loading folders, tags and collections',
    interface: 'Preparing your workspace',
    ready: 'Ready'
};
