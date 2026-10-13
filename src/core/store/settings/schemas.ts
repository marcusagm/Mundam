import { z } from 'zod';

/**
 * Schema for appearance preferences.
 */
export const AppearancePayloadSchema = z.object({
    mode: z.enum(['dark', 'light', 'system']).optional(),
    theme: z
        .enum([
            'neutral',
            'blue',
            'emerald',
            'orange',
            'rose',
            'violet',
            'teal',
            'zinc',
            'indigo',
            'fuchsia',
            'slate',
            'stone'
        ])
        .optional(),
    radius: z.number().min(0).max(20).optional(),
    fontSize: z.enum(['small', 'medium', 'large']).optional()
});

export type AppearancePayload = z.infer<typeof AppearancePayloadSchema>;

/**
 * Schema for general application settings.
 */
export const SettingsPayloadSchema = z.object({
    thumbnailThreads: z.number().min(0).max(32).optional(),
    cacheRetentionDays: z.number().min(1).max(365).optional(),
    historyLimit: z.number().min(1).max(1000).optional(),
    indexerConcurrencyLimit: z.number().min(10).max(500).optional(),
    autoDetectDuplicates: z.boolean().optional()
});

export type SettingsPayload = z.infer<typeof SettingsPayloadSchema>;

/**
 * Schema for cache management actions.
 */
export const CacheCleanupSchema = z.object({
    maxAgeDays: z.number().min(0).max(365).optional()
});

export type CacheCleanupPayload = z.infer<typeof CacheCleanupSchema>;
