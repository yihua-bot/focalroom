// G7 Publication Panel：桌面端出版物管理面板。
// 创建出版物、从序列生成初稿、查看页面列表。
import { useEffect, useState } from 'react';
import { Button } from '../../components/shared/Button';

import type { PublicationInfo, PublicationPage, PublicationRepository } from '../../domain/publication';

export function PublicationPanel({ repository, projectId, sequenceCount, readOnly = false }: {
  repository?: PublicationRepository;
  projectId: string;
  sequenceCount: number;
  readOnly?: boolean;
}) {
  const [publications, setPublications] = useState<PublicationInfo[]>([]);
  const [creating, setCreating] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [title, setTitle] = useState('');
  const [perPage, setPerPage] = useState(1);
  const [notice, setNotice] = useState('');
  const [pages, setPages] = useState<Record<string, PublicationPage[]>>({});
  const [loadingPages, setLoadingPages] = useState<string | null>(null);

  const load = async () => {
    if (!repository) return;
    const list = await repository.list(projectId);
    setPublications(list);
  };

  useEffect(() => {
    let active = true;
    if (repository) {
      repository.list(projectId).then(list => {
        if (active) setPublications(list);
      }).catch(error => { if (active) setNotice(error instanceof Error ? error.message : '读取出版物失败'); });
    }
    return () => { active = false; };
  }, [repository, projectId]);

  const create = async () => {
    if (!repository || readOnly || creating || !title.trim()) return;
    setCreating(true);
    setNotice('');
    try {
      await repository.create(projectId, title.trim());
      setTitle('');
      await load();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : '创建出版物失败');
    } finally { setCreating(false); }
  };

  const generate = async (pubId: string) => {
    if (!repository || readOnly || generating) return;
    setGenerating(true);
    setNotice('');
    try {
      const pages = await repository.generate(projectId, pubId, perPage);
      setNotice(`已生成 ${pages} 页初稿`);
      setPages(current => ({ ...current, [pubId]: [] }));
    } catch (e) {
      setNotice(e instanceof Error ? e.message : '生成失败');
    } finally { setGenerating(false); }
  };

  const showPages = async (publicationId: string) => {
    if (!repository || loadingPages) return;
    setLoadingPages(publicationId);
    setNotice('');
    try {
      const loaded = await repository.pages(projectId, publicationId);
      setPages(current => ({ ...current, [publicationId]: loaded }));
      if (!loaded.length) setNotice('还没有页面，请先从作品序列生成。');
    } catch (error) {
      setNotice(error instanceof Error ? error.message : '读取页面失败');
    } finally { setLoadingPages(null); }
  };

  if (!repository) {
    return <div className="publication-empty"><p>出版功能需要桌面版。</p></div>;
  }

  return <section className="publication-panel">
    <h3>出版物</h3>
    <p className="micro">从作品序列生成摄影书/展览初稿。</p>

    <div className="publication-create">
      <input
        type="text"
        value={title}
        placeholder="出版物标题…"
        maxLength={200}
        disabled={creating || readOnly}
        aria-label="出版物标题"
        onChange={e => setTitle(e.target.value)}
        onKeyDown={e => { if (e.key === 'Enter') void create(); }}
      />
      <Button size="small" disabled={creating || readOnly || !title.trim()} onClick={() => void create()}>
        {creating ? '创建中…' : '创建'}
      </Button>
    </div>

    {publications.map(pub => (
      <div key={pub.id} className="publication-card">
        <div className="publication-card-head">
          <strong>{pub.title}</strong>
          <span className={`publication-state ${pub.state}`}>{{ draft: '草稿', in_review: '审阅中', finalized: '已定稿', delivered: '已交付' }[pub.state] ?? pub.state}</span>
        </div>
        <div className="publication-card-meta">
          {{ photobook: '摄影书', exhibition: '展览', portfolio: '作品集' }[pub.pubType] ?? pub.pubType} · {new Date(pub.createdAt).toLocaleDateString()}
        </div>
        <div className="publication-actions">
          <select aria-label={`${pub.title}每页照片数`} value={perPage} disabled={readOnly || generating} onChange={e => setPerPage(Number(e.target.value))}>
            <option value={1}>每页 1 张</option>
            <option value={2}>每页 2 张</option>
            <option value={4}>每页 4 张</option>
          </select>
          <Button size="small" disabled={readOnly || generating || sequenceCount === 0 || pub.state !== 'draft'} onClick={() => void generate(pub.id)}>
            {generating ? '生成中…' : `从序列生成（${sequenceCount} 张）`}
          </Button>
          <Button size="small" disabled={!!loadingPages || generating} onClick={() => void showPages(pub.id)}>
            {loadingPages === pub.id ? '读取中…' : '查看页面'}
          </Button>
        </div>
        {!!pages[pub.id]?.length && <div className="publication-pages" aria-label={`${pub.title}页面初稿`}>
          {pages[pub.id].map(page => <figure key={page.id} className="publication-page-preview">
            <div className="publication-paper" style={{ aspectRatio: `${page.widthMm} / ${page.heightMm}` }}>
              {page.frames.map(frame => <div key={frame.id} className="publication-frame" style={{
                left: `${frame.xMm / page.widthMm * 100}%`, top: `${frame.yMm / page.heightMm * 100}%`,
                width: `${frame.widthMm / page.widthMm * 100}%`, height: `${frame.heightMm / page.heightMm * 100}%`,
              }}>{frame.thumbnailSrc ? <img src={frame.thumbnailSrc} alt="" style={{ objectFit: frame.fitMode === 'fit' ? 'contain' : frame.fitMode === 'fill' ? 'cover' : 'fill' }} />
                : <span>照片暂不可用</span>}</div>)}
            </div>
            <figcaption>{page.pageNumber === 0 ? '封面' : `第 ${page.pageNumber} 页`}</figcaption>
          </figure>)}
        </div>}
      </div>
    ))}

    {publications.length === 0 && (
      <p className="publication-hint">还没有出版物。创建一个，然后从作品序列自动生成页面。</p>
    )}

    {notice && <p role="status" className="publication-notice">{notice}</p>}
  </section>;
}
