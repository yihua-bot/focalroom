import { zh } from '../../app/copy';
import type { KnowledgeMatch } from '../../domain/knowledgeMatch';

/**
 * 参考来源与判断方法共用同一套"为什么算这条"的说法。
 * 词面命中、点名取用的自带方法、云端服务命中是三件不同的事，不能混成一句"相关资料"。
 */
export function knowledgeMatchLines(reason?: KnowledgeMatch | null): string[] {
  if (!reason) return [];
  if (reason.kind === 'builtin_method') return [zh.chatMatchBuiltin];
  if (reason.kind === 'cloud_search') {
    const score = reason.relevance == null ? zh.chatMatchCloudNoScore : zh.chatMatchCloudRelevance.replace('{value}', String(reason.relevance));
    return [`${zh.chatMatchCloud} · ${score}`];
  }
  const terms = reason.matchedTerms.join('、');
  const more = reason.termCount > reason.matchedTerms.length ? zh.chatMatchTermsTruncated.replace('{shown}', String(reason.matchedTerms.length)) : '';
  const fields = reason.fields.map(field => zh[`chatMatchField_${field}`]).join('、');
  const lines = [`${zh.chatMatchLocal.replace('{count}', String(reason.termCount)).replace('{terms}', terms)}${more} · ${zh.chatMatchFields.replace('{fields}', fields)}`];
  if (reason.matched != null && reason.returned != null && reason.matched > reason.returned) {
    lines.push(zh.chatMatchTruncated.replace('{matched}', String(reason.matched)).replace('{returned}', String(reason.returned)));
  }
  return lines;
}