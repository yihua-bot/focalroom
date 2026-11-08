import { zh } from '../../app/copy';
import type { PreviewRepairOutcome } from '../../domain/models';

// 修坏掉的本地中间文件这个动作的结局说法，AI 面板、两处审阅抽屉和照片面板共用同一套词，避免四处各讲一套。
export function previewRepairLabel(outcome?: string): string {
  if (!outcome) return zh.chatPreviewRepair_failed;
  const key = `chatPreviewRepair_${outcome}`;
  return zh[key] === key ? outcome : zh[key];
}

/**
 * 修完之后这张的本地文件状态怎么说。只有后端确认文件读得出的两个结局才改口；
 * 原图不在、原图换过、路径可疑、这一步失败都保持原来的说法，不能把"没修成"显示成"现在好了"。
 */
export function previewStatusAfterRepair(outcome?: string): string | null {
  return outcome === 'repaired' || outcome === 'healthy' ? zh.chatImageVersionRestored : null;
}

/** 按照片记下后端给的原始结局：改口要看结局本身，看翻译后的句子会看不准。 */
export function outcomeByPhoto(results: PreviewRepairOutcome[]): Record<string, string> {
  return Object.fromEntries(results.map(result => [result.photoId, result.outcome]));
}
