import { describe, expect, it } from 'vitest';
import { MatchingPublicationTracker } from './matchingPublication';

describe('durable duplicate publication reconciliation', () => {
    it('recovers missed results and ignores repeated activity samples', () => {
        const publications = new MatchingPublicationTracker();
        expect(publications.observe('session', 4)).toBe(true);
        expect(publications.observe('session', 4)).toBe(false);
        expect(publications.observe('session', 5)).toBe(true);
        expect(publications.observe('session', 5)).toBe(false);
    });

    it('does not invalidate again after a delivered result event', () => {
        const publications = new MatchingPublicationTracker();
        publications.observe('session', 0);
        publications.acknowledge(3);
        expect(publications.observe('session', 3)).toBe(false);
        expect(publications.observe('session', 4)).toBe(true);
        publications.acknowledge(2);
        expect(publications.observe('session', 4)).toBe(false);
    });

    it('reconciles a restarted service even when the restored counter is lower', () => {
        const publications = new MatchingPublicationTracker();
        publications.observe('old', 8);
        expect(publications.observe('new', 2)).toBe(true);
        expect(publications.observe('new', 2)).toBe(false);
        expect(publications.observe('new', 3)).toBe(true);
    });

    it('cannot let an event from an older restored catalog suppress future publications', () => {
        const publications = new MatchingPublicationTracker();
        publications.observe('old', 30);
        publications.observe('new', 2);
        publications.acknowledge(30);
        expect(publications.observe('new', 3)).toBe(true);
        expect(publications.observe('new', 4)).toBe(true);
    });
});
