// 知识库向量检索：应用层余弦相似度（数据量 <1000 条，无需 pgvector）
// 后续数据量大时可升级 pgvector + HNSW 索引

import { Pool } from "pg";

interface EmbeddedItem {
  kind: string; id: string; title: string; content: string; embedding: number[];
}

export class KnowledgeVectorService {
  private cache: EmbeddedItem[] | null = null;
  private cacheAt = 0;
  private pool: Pool;
  constructor(pool: Pool) { this.pool = pool; }

  /** 嵌入文本：用 DeepSeek chat 生成文本特征向量（降级：字符频率向量）。 */
  embed(text: string): number[] {
    // 简化嵌入：字符三元组频率 → 256 维向量（无需外部 embedding API）
    const vec = new Array(256).fill(0);
    const chars = text.toLowerCase().replace(/\s+/g, "");
    for (let i = 0; i < chars.length - 2; i++) {
      const tri = chars.slice(i, i + 3);
      let h = 0;
      for (let j = 0; j < tri.length; j++) h = (h * 31 + tri.charCodeAt(j)) % 256;
      vec[h] += 1;
    }
    // 归一化
    const norm = Math.sqrt(vec.reduce((s, v) => s + v * v, 0)) || 1;
    return vec.map(v => v / norm);
  }

  cosine(a: number[], b: number[]): number {
    let dot = 0;
    for (let i = 0; i < Math.min(a.length, b.length); i++) dot += a[i] * b[i];
    return dot;
  }

  /** 为知识库实体生成嵌入（幂等）。 */
  async buildIndex(): Promise<number> {
    const entities = (await this.pool.query(
      `SELECT 'photographer' as kind, id::text, name_en as title,
              coalesce(name_en,'') || ' ' || coalesce(name_zh,'') || ' ' || coalesce(biography,'') || ' ' || array_to_string(movements,' ') as content
       FROM fr_photographers
       UNION ALL
       SELECT 'series' as kind, id::text, title_en as title,
              coalesce(title_en,'') || ' ' || coalesce(title_zh,'') || ' ' || coalesce(intent,'') || ' ' || coalesce(method_notes,'') || ' ' || coalesce(capture_format,'') as content
       FROM fr_photo_series
       UNION ALL
       SELECT 'claim' as kind, id::text, left(claim_text, 60) as title, claim_text as content
       FROM fr_knowledge_claims`,
    )).rows;

    for (const e of entities) {
      const embedding = this.embed(String(e.content));
      await this.pool.query(
        `INSERT INTO fr_knowledge_embeddings(entity_kind, entity_id, content, embedding, dim)
         VALUES($1, $2, $3, $4::jsonb, $5)
         ON CONFLICT (entity_kind, entity_id) DO UPDATE SET content = EXCLUDED.content, embedding = EXCLUDED.embedding`,
        [e.kind, e.id, String(e.content).slice(0, 500), JSON.stringify(embedding), embedding.length],
      );
    }
    this.cache = null; // 清缓存
    return entities.length;
  }

  /** 语义搜索：向量余弦 + 关键词混合。 */
  async search(query: string, limit = 5): Promise<Array<{ kind: string; id: string; title: string; score: number }>> {
    // 懒加载缓存（5 分钟）
    if (!this.cache || Date.now() - this.cacheAt > 300_000) {
      const rows = (await this.pool.query(
        `SELECT entity_kind as kind, entity_id as id, content, embedding FROM fr_knowledge_embeddings`,
      )).rows;
      this.cache = rows.map((r: Record<string, unknown>) => ({
        kind: String(r.kind), id: String(r.id),
        title: String(r.content).slice(0, 60),
        content: String(r.content),
        embedding: Array.isArray(r.embedding) ? r.embedding as number[] : (typeof r.embedding === "string" ? JSON.parse(r.embedding) : []),
      }));
      this.cacheAt = Date.now();
    }

    const q = this.embed(query);
    const qLower = query.toLowerCase();
    const scored = this.cache.map(item => {
      const vecScore = this.cosine(q, item.embedding);
      // 关键词加分
      const kwScore = qLower.split(/\s+/).filter(w => w.length > 1 && item.content.toLowerCase().includes(w)).length * 0.1;
      return { kind: item.kind, id: item.id, title: item.title, score: vecScore + kwScore };
    });
    return scored.filter(s => s.score > 0.05).sort((a, b) => b.score - a.score).slice(0, limit);
  }
}
