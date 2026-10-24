import { convertFileSrc } from '@tauri-apps/api/core';
import { z } from 'zod';

export const historicalObservationSchema = z.object({ photoId:z.string(),photoHash:z.string().regex(/^[a-f0-9]{64}$/i),photoName:z.string(),thumbnailPath:z.string().nullable(),modelId:z.string(),promptVersion:z.string(),taskId:z.string(),callId:z.string(),visibleFacts:z.array(z.string()),interpretation:z.string(),uncertainty:z.string(),currentHashMatches:z.boolean() });
export const withLocalRevisionPreviews = <T extends { currentHashMatches: boolean; thumbnailPath: string | null }>(photos: T[]): T[] => photos.map(photo => ({ ...photo, thumbnailPath: photo.currentHashMatches && photo.thumbnailPath ? convertFileSrc(photo.thumbnailPath) : null }));
export const selectionRevisionSchema = z.object({
  draftId:z.string(),parent:z.object({kind:z.literal('selection'),id:z.string(),revision:z.number().int().positive()}),parentTitle:z.string(),parentSummary:z.string(),parentState:z.enum(['pending','applied','discarded']),parentBaseRevision:z.number().int().nonnegative(),
  items:z.array(z.object({photoId:z.string(),photoHash:z.string().regex(/^[a-f0-9]{64}$/i),photoName:z.string(),position:z.number().int().nonnegative(),action:z.enum(['keep','reject','undecided']),rationale:z.string(),currentHashMatches:z.boolean(),thumbnailPath:z.string().nullable()})).max(30),
  feedback:z.array(z.object({messageId:z.number().int().positive(),role:z.enum(['user','assistant']),content:z.string(),createdAt:z.string()})).max(30),
  reviews:z.array(z.object({commandId:z.string(),kind:z.enum(['apply','reorder','rebase','discard']),createdAt:z.string(),mode:z.enum(['partial','full']).nullable(),photoIds:z.array(z.string()),photoNames:z.array(z.string()),partial:z.boolean(),undoneAtDelegation:z.boolean().nullable()})).max(30),
  observations:z.array(historicalObservationSchema).max(30),totalItems:z.number().int().nonnegative(),totalFeedback:z.number().int().nonnegative(),totalReviews:z.number().int().nonnegative(),totalObservations:z.number().int().nonnegative(),nextOffset:z.number().int().nonnegative().nullable(),hasEarlier:z.boolean(),
});
