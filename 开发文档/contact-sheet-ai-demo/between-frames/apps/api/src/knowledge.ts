import type { Pool } from 'pg';
import { z } from 'zod';
import type { AuthService, Headers } from './auth.ts';
import { assert } from './errors.ts';

const searchQuery = z.string().trim().min(1).max(200);
const movementFilter = z.string().trim().min(1).max(100).optional();
const entityKind = z.enum(['photographer', 'series', 'artwork', 'photobook', 'claim']).optional();

export interface KnowledgeSearchResult {
  kind: 'photographer' | 'series' | 'artwork' | 'photobook';
  id: string;
  title: string;
  subtitle: string | null;
  photographer: string | null;
  reviewStatus: string;
  relevance: number;
}

/**
 * K0/K3 Focal Photography Intelligence — 摄影知识公共检索。
 * 只读服务：所有终端（Mac/Web/iPhone/iPad/Android）共享同一套知识底座。
 * 与项目创作知识（fr_research_references）和审美记忆（author_memory）权限隔离。
 */
export class KnowledgeService {
  readonly pool: Pool;
  readonly auth: AuthService;
  constructor(pool: Pool, auth: AuthService) { this.pool = pool; this.auth = auth; }

  /** 跨实体搜索：摄影师/系列/作品/摄影书，按相关度排序。 */
  async search(headers: Headers, query: unknown, kind: unknown, movement: unknown) {
    await this.auth.authenticate(headers, false);
    const q = searchQuery.parse(query);
    const filterKind = entityKind.parse(kind);
    const filterMovement = movementFilter.parse(movement);
    const results: KnowledgeSearchResult[] = [];
    const like = `%${q}%`;

    if (!filterKind || filterKind === 'photographer') {
      const rows = (await this.pool.query(
        `SELECT id, name_en, name_zh, nationality, movements, review_status
         FROM fr_photographers
         WHERE ($1::text IS NULL OR EXISTS (SELECT 1 FROM unnest(movements) m WHERE m ILIKE $2))
           AND (name_en ILIKE $2 OR coalesce(name_zh,'') ILIKE $2 OR coalesce(biography,'') ILIKE $2
                OR array_to_string(movements, ' ') ILIKE $2)
         ORDER BY name_en LIMIT 20`,
        [filterMovement ?? null, like],
      )).rows;
      for (const row of rows) results.push({
        kind: 'photographer', id: row.id, title: row.name_en,
        subtitle: [row.name_zh, row.nationality].filter(Boolean).join(' · ') || null,
        photographer: null, reviewStatus: row.review_status,
        relevance: row.name_en.toLowerCase().includes(q.toLowerCase()) ? 1 : 0.5,
      });
    }

    if (!filterKind || filterKind === 'series') {
      const rows = (await this.pool.query(
        `SELECT s.id, s.title_en, s.title_zh, s.start_year, s.end_year, s.review_status, p.name_en AS photographer
         FROM fr_photo_series s JOIN fr_photographers p ON p.id = s.photographer_id
         WHERE s.title_en ILIKE $1 OR coalesce(s.title_zh,'') ILIKE $1 OR coalesce(s.intent,'') ILIKE $1
         ORDER BY s.title_en LIMIT 20`, [like],
      )).rows;
      for (const row of rows) results.push({
        kind: 'series', id: row.id, title: row.title_en,
        subtitle: [row.title_zh, row.start_year ? `${row.start_year}–${row.end_year ?? ''}` : null].filter(Boolean).join(' · ') || null,
        photographer: row.photographer, reviewStatus: row.review_status,
        relevance: 0.5,
      });
    }

    if (!filterKind || filterKind === 'artwork') {
      const rows = (await this.pool.query(
        `SELECT a.id, a.title_en, a.title_zh, a.capture_date, a.review_status, p.name_en AS photographer
         FROM fr_artworks a JOIN fr_photographers p ON p.id = a.photographer_id
         WHERE a.title_en ILIKE $1 OR coalesce(a.title_zh,'') ILIKE $1
            OR array_to_string(a.subjects, ' ') ILIKE $1
            OR array_to_string(a.visual_concepts, ' ') ILIKE $1
         ORDER BY a.title_en LIMIT 20`, [like],
      )).rows;
      for (const row of rows) results.push({
        kind: 'artwork', id: row.id, title: row.title_en,
        subtitle: row.title_zh ?? row.capture_date ?? null,
        photographer: row.photographer, reviewStatus: row.review_status,
        relevance: 0.4,
      });
    }

    if (!filterKind || filterKind === 'photobook') {
      const rows = (await this.pool.query(
        `SELECT b.id, b.title, b.publication_year, b.review_status, p.name_en AS photographer
         FROM fr_photobooks b LEFT JOIN fr_photographers p ON p.id = b.photographer_id
         WHERE b.title ILIKE $1 OR coalesce(b.publisher,'') ILIKE $1 OR coalesce(b.structure_notes,'') ILIKE $1
         ORDER BY b.title LIMIT 20`, [like],
      )).rows;
      for (const row of rows) results.push({
        kind: 'photobook', id: row.id, title: row.title,
        subtitle: row.publication_year ? String(row.publication_year) : null,
        photographer: row.photographer, reviewStatus: row.review_status,
        relevance: 0.3,
      });
    }

    return results.sort((a, b) => b.relevance - a.relevance).slice(0, 50);
  }

  /** 摄影师详情 + 关联系列 + 事实断言 + 知识来源。 */
  async photographer(headers: Headers, id: unknown) {
    await this.auth.authenticate(headers, false);
    const photographerId = z.uuid().parse(id);
    const photographer = (await this.pool.query(
      `SELECT * FROM fr_photographers WHERE id = $1`, [photographerId],
    )).rows[0];
    assert(photographer, 404, 'NOT_FOUND', '摄影师不存在');
    const series = (await this.pool.query(
      `SELECT * FROM fr_photo_series WHERE photographer_id = $1 ORDER BY coalesce(start_year, 9999), title_en`, [photographerId],
    )).rows;
    const claims = (await this.pool.query(
      `SELECT c.* FROM fr_knowledge_claims c
       WHERE (c.entity_type = 'photographer' AND c.entity_id = $1)
          OR (c.entity_type = 'series' AND c.entity_id IN (SELECT id FROM fr_photo_series WHERE photographer_id = $1))
       ORDER BY c.created_at`, [photographerId],
    )).rows;
    const sources = (await this.pool.query(
      `SELECT * FROM fr_knowledge_sources WHERE entity_type = 'photographer' AND entity_id = $1`, [photographerId],
    )).rows;
    const relations = (await this.pool.query(
      `SELECT * FROM fr_knowledge_relations WHERE (from_type='photographer' AND from_id=$1) OR (to_type='photographer' AND to_id=$1)`, [photographerId],
    )).rows;
    return { ...photographer, series, claims, sources, relations };
  }

  /** 作品详情 + 事实断言 + 来源。 */
  async artwork(headers: Headers, id: unknown) {
    await this.auth.authenticate(headers, false);
    const artworkId = z.uuid().parse(id);
    const artwork = (await this.pool.query(
      `SELECT a.*, p.name_en AS photographer_name, s.title_en AS series_title
       FROM fr_artworks a
       JOIN fr_photographers p ON p.id = a.photographer_id
       LEFT JOIN fr_photo_series s ON s.id = a.series_id
       WHERE a.id = $1`, [artworkId],
    )).rows[0];
    assert(artwork, 404, 'NOT_FOUND', '作品不存在');
    const claims = (await this.pool.query(
      `SELECT * FROM fr_knowledge_claims WHERE entity_type = 'artwork' AND entity_id = $1 ORDER BY created_at`, [artworkId],
    )).rows;
    const sources = (await this.pool.query(
      `SELECT * FROM fr_knowledge_sources WHERE entity_type = 'artwork' AND entity_id = $1`, [artworkId],
    )).rows;
    return { ...artwork, claims, sources };
  }
}
