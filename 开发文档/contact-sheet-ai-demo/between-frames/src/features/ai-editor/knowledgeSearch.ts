import { zh } from '../../app/copy';
import type { KnowledgeSearch } from '../../domain/knowledgeSearch';

/**
 * 检索台账的说法：取到了、确实没有、服务没答上来、这次没跑成，四件事分开讲。
 * 第一行是结论，后面几行是查过的词与次数；结论不许拿"没有资料"去盖住一次失败。
 */
export function knowledgeSearchLines(search: KnowledgeSearch): string[] {
  const localOnly = search.tool === 'search_photography_knowledge' && search.cloudAvailable == null;
  // 只有通用知识检索才同时查本地札记与云端；另外三种工具根本没有本地语料，
  // 不能替它们说"本地也没有"。
  const noMatch = localOnly
    ? zh.chatSearchNoMatchLocal
    : search.tool === 'search_photography_knowledge' ? zh.chatSearchNoMatch : zh.chatSearchNoMatchCloud;
  const outcome = search.outcome === 'found'
    ? zh.chatSearchFound.replace('{count}', String(search.returned))
    : search.outcome === 'unavailable'
      ? zh.chatSearchUnavailable
      : search.outcome === 'failed'
        ? zh.chatSearchFailed
        : noMatch;
  const lines = [outcome, search.query ? zh.chatSearchAsked.replace('{query}', search.query) : zh.chatSearchNoQuery];
  if (search.localMatched != null) lines.push(zh.chatSearchLocalMatched.replace('{count}', String(search.localMatched)));
  if (search.cloudError) lines.push(zh.chatSearchCloudError.replace('{message}', search.cloudError.message));
  if (search.error) lines.push(zh.chatSearchFailedError.replace('{message}', search.error.message));
  return lines;
}

export function knowledgeSearchToolLabel(tool: KnowledgeSearch['tool']): string {
  return zh[`chatSearchTool_${tool}`];
}