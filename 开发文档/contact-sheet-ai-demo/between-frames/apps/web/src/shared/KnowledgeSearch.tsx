import { useState } from "react";
import { request, errorMessage as message, type Session } from "../shared-api";

/** K4 Focal Photography Intelligence — 摄影研究检索（Explore）。
 * 跨实体搜索知识库：摄影师/系列/作品/摄影书；结果可展开查看事实断言与来源。 */
type KnowledgeSearchResult = {
  kind: "photographer" | "series" | "artwork" | "photobook";
  id: string;
  title: string;
  subtitle: string | null;
  photographer: string | null;
  reviewStatus: string;
  relevance: number;
};

type KnowledgeClaim = {
  claim_type: string;
  claim_text: string;
  review_status: string;
};

type KnowledgeSource = {
  url: string;
  institution: string;
};

type KnowledgeDetail = {
  id: string;
  title_en?: string;
  name_en?: string;
  title?: string;
  birth_year?: number | null;
  death_year?: number | null;
  movement?: string | null;
  claims?: KnowledgeClaim[];
  sources?: KnowledgeSource[];
  series?: Array<{ id: string; title_en: string; year_start?: number | null }>;
  artworks?: Array<{ id: string; title_en: string; capture_date?: string | null }>;
};

const KIND_LABEL: Record<KnowledgeSearchResult["kind"], string> = {
  photographer: "摄影师",
  series: "系列",
  artwork: "作品",
  photobook: "摄影书",
};

function detailPath(kind: KnowledgeSearchResult["kind"], id: string): string {
  switch (kind) {
    case "photographer": return `/knowledge/photographers/${id}`;
    case "series": return `/knowledge/series/${id}`;
    case "artwork": return `/knowledge/artworks/${id}`;
    case "photobook": return `/knowledge/photobooks/${id}`;
  }
}

export function KnowledgeSearch({ session }: { session: Session }) {
  const [query, setQuery] = useState("");
  const [submitted, setSubmitted] = useState("");
  const [kindFilter, setKindFilter] = useState<string>("");
  const [results, setResults] = useState<KnowledgeSearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [selected, setSelected] = useState<KnowledgeSearchResult | null>(null);
  const [detail, setDetail] = useState<KnowledgeDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  const runSearch = async (q: string, kind: string) => {
    setLoading(true);
    setError("");
    try {
      const params = new URLSearchParams({ q });
      if (kind) params.set("kind", kind);
      const data = await request<KnowledgeSearchResult[]>(`/knowledge/search?${params}`, session);
      setResults(Array.isArray(data) ? data : []);
    } catch (e) {
      setError(message(e));
    } finally {
      setLoading(false);
    }
  };

  const open = async (item: KnowledgeSearchResult) => {
    setSelected(item);
    setDetail(null);
    setDetailLoading(true);
    setError("");
    try {
      const data = await request<KnowledgeDetail>(detailPath(item.kind, item.id), session);
      setDetail(data);
    } catch (e) {
      setError(message(e));
    } finally {
      setDetailLoading(false);
    }
  };

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    const q = query.trim();
    if (!q) return;
    setSelected(null);
    setDetail(null);
    setSubmitted(q);
    void runSearch(q, kindFilter);
  };

  const applyFilter = (kind: string) => {
    setKindFilter(kind);
    setSelected(null);
    setDetail(null);
    if (submitted) void runSearch(submitted, kind);
  };

  return (
    <div className="fr-knowledge">
      <header className="fr-knowledge-header">
        <h1>摄影研究</h1>
        <p>检索摄影知识库——摄影师、系列、作品与摄影书，每条事实均标注来源。</p>
        <form onSubmit={submit} className="fr-knowledge-form">
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="搜索：Stephen Shore、The Americans、新彩色摄影…"
            aria-label="知识库搜索"
          />
          <button type="submit" disabled={!query.trim() || loading}>
            {loading ? "搜索中…" : "搜索"}
          </button>
        </form>
        <div className="fr-knowledge-filters" role="tablist">
          {[
            { value: "", label: "全部" },
            { value: "photographer", label: "摄影师" },
            { value: "series", label: "系列" },
            { value: "artwork", label: "作品" },
            { value: "photobook", label: "摄影书" },
          ].map((option) => (
            <button
              key={option.value || "all"}
              role="tab"
              aria-selected={kindFilter === option.value}
              className={kindFilter === option.value ? "active" : ""}
              onClick={() => applyFilter(option.value)}
              type="button"
            >{option.label}</button>
          ))}
        </div>
      </header>

      {error && <p className="fr-knowledge-error" role="alert">{error}</p>}

      {submitted && !loading && results.length === 0 && !error && (
        <p className="fr-knowledge-empty">没有找到与「{submitted}」相关的条目。</p>
      )}

      <div className="fr-knowledge-body">
        <ul className="fr-knowledge-results">
          {results.map((item) => (
            <li key={`${item.kind}-${item.id}`}>
              <button
                type="button"
                className={selected?.id === item.id && selected?.kind === item.kind ? "active" : ""}
                onClick={() => void open(item)}
              >
                <span className="fr-knowledge-kind" data-kind={item.kind}>{KIND_LABEL[item.kind]}</span>
                <span className="fr-knowledge-title">{item.title}</span>
                {item.subtitle && <span className="fr-knowledge-subtitle">{item.subtitle}</span>}
                {item.photographer && <span className="fr-knowledge-photographer">{item.photographer}</span>}
              </button>
            </li>
          ))}
        </ul>

        {selected && (
          <section className="fr-knowledge-detail" aria-label={`${KIND_LABEL[selected.kind]}详情`}>
            {detailLoading && <p>加载中…</p>}
            {detail && (
              <>
                <h2>{detail.name_en ?? detail.title_en ?? detail.title ?? selected.title}</h2>
                {"birth_year" in detail && detail.birth_year != null && (
                  <p className="fr-knowledge-meta">
                    {detail.birth_year}–{detail.death_year ?? ""}
                    {detail.movement ? ` · ${detail.movement}` : ""}
                  </p>
                )}
                {detail.series && detail.series.length > 0 && (
                  <div>
                    <h3>系列</h3>
                    <ul className="fr-knowledge-list">
                      {detail.series.map((s) => (
                        <li key={s.id}>{s.title_en}{s.year_start ? `（${s.year_start}–）` : ""}</li>
                      ))}
                    </ul>
                  </div>
                )}
                {detail.artworks && detail.artworks.length > 0 && (
                  <div>
                    <h3>作品</h3>
                    <ul className="fr-knowledge-list">
                      {detail.artworks.map((a) => (
                        <li key={a.id}>{a.title_en}{a.capture_date ? ` · ${a.capture_date}` : ""}</li>
                      ))}
                    </ul>
                  </div>
                )}
                {detail.claims && detail.claims.length > 0 && (
                  <div>
                    <h3>事实断言</h3>
                    <ul className="fr-knowledge-claims">
                      {detail.claims.map((claim, index) => (
                        <li key={index}>
                          <span className="fr-knowledge-claim-type" data-status={claim.review_status}>{claim.claim_type}</span>
                          <span>{claim.claim_text}</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
                {detail.sources && detail.sources.length > 0 && (
                  <div>
                    <h3>来源</h3>
                    <ul className="fr-knowledge-list">
                      {detail.sources.map((source, index) => (
                        <li key={index}>
                          <a href={source.url} target="_blank" rel="noreferrer">{source.institution}</a>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </>
            )}
          </section>
        )}
      </div>
    </div>
  );
}
