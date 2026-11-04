/** Reconciles result publications without treating every activity poll as invalidation. */
export class MatchingPublicationTracker {
    private instance: string | undefined;
    private sequence = 0;
    private acknowledged: number | undefined;

    /** A delivered result event already caused a fresh group query. */
    acknowledge(sequence: number | undefined): void {
        if (sequence !== undefined && Number.isFinite(sequence))
            this.acknowledged = Math.max(this.acknowledged ?? 0, sequence);
    }

    /** Returns true for a missed publication or a restarted service, then advances the baseline. */
    observe(instance: string, sequence: number): boolean {
        const restarted = this.instance !== undefined && this.instance !== instance;
        const changed = restarted || (sequence > this.sequence && sequence !== this.acknowledged);
        this.instance = instance;
        this.sequence = restarted ? sequence : Math.max(this.sequence, sequence);
        this.acknowledged = undefined;
        return changed;
    }
}
