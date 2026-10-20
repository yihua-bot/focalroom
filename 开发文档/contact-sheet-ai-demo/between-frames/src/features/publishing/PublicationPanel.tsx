// G7 Publication Panel：桌面端出版物管理面板。
// 创建出版物、从序列生成初稿、查看页面列表。
import { useEffect, useState } from 'react';
import { Button } from '../../components/shared/Button';

interface PublicationInfo {
  id: string;
  projectId: string;
  pubType: string;
  title: string;
  state: string;
  createdAt: string;
}

export interface PublicationRepository {
  list(projectId: string): Promise<PublicationInfo[]>;
  create(projectId: string, title: string, pubType?: string): Promise<PublicationInfo>;
  generate(projectId: string, publicationId: string, photosPerPage?: number): Promise<number>;
}

export function PublicationPanel({ repository, projectId, sequenceCount }: {
  repository?: { publication: PublicationRepository };
  projectId: string;
  sequenceCount: number;
}) {
  const [publications, setPublications] = useState<PublicationInfo[]>([]);
  const [creating, setCreating] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [title, setTitle] = useState('');
  const [perPage, setPerPage] = useState(1);
  const [notice, setNotice] = useState('');

  const load = async () => {
    if (!repository) return;
    const list = await repository.publication.list(projectId);
    setPublications(list);
  };

  useEffect(() => {
    let active = true;
    if (repository) {
      repository.publication.list(projectId).then(list => {
        if (active) setPublications(list);
      }).catch(() => {});
    }
    return () => { active = false; };
  }, [repository, projectId]);

  const create = async () => {
    if (!repository || creating || !title.trim()) return;
    setCreating(true);
    try {
      await repository.publication.create(projectId, title.trim());
      setTitle('');
      await load();
    } finally { setCreating(false); }
  };

  const generate = async (pubId: string) => {
    if (!repository || generating) return;
    setGenerating(true);
    setNotice('');
    try {
      const pages = await repository.publication.generate(projectId, pubId, perPage);
      setNotice(`已生成 ${pages} 页初稿`);
    } catch (e) {
      setNotice(e instanceof Error ? e.message : '生成失败');
    } finally { setGenerating(false); }
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
        disabled={creating}
        onChange={e => setTitle(e.target.value)}
        onKeyDown={e => { if (e.key === 'Enter') void create(); }}
      />
      <Button size="small" disabled={creating || !title.trim()} onClick={() => void create()}>
        {creating ? '创建中…' : '创建'}
      </Button>
    </div>

    {publications.map(pub => (
      <div key={pub.id} className="publication-card">
        <div className="publication-card-head">
          <strong>{pub.title}</strong>
          <span className={`publication-state ${pub.state}`}>{pub.state}</span>
        </div>
        <div className="publication-card-meta">
          {pub.pubType} · {new Date(pub.createdAt).toLocaleDateString()}
        </div>
        <div className="publication-actions">
          <select value={perPage} onChange={e => setPerPage(Number(e.target.value))}>
            <option value={1}>每页 1 张</option>
            <option value={2}>每页 2 张</option>
            <option value={4}>每页 4 张</option>
          </select>
          <Button size="small" disabled={generating || sequenceCount === 0} onClick={() => void generate(pub.id)}>
            {generating ? '生成中…' : `从序列生成（${sequenceCount} 张）`}
          </Button>
        </div>
      </div>
    ))}

    {publications.length === 0 && (
      <p className="publication-hint">还没有出版物。创建一个，然后从作品序列自动生成页面。</p>
    )}

    {notice && <p role="status" className="publication-notice">{notice}</p>}
  </section>;
}
