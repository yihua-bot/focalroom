import type { PublicationState } from '../../domain/publication';
import { zh } from '../../app/copy';

// 服务端 publication_commands.rs 里那份流转表是唯一裁决者；这里只是它的一份镜像，
// 作用是在按钮还没按下去之前少给一个非法选项。改状态机先改 Rust，再照抄到这里。
const transitions: Record<PublicationState, readonly PublicationState[]> = {
  draft: ['in_review'],
  in_review: ['draft', 'finalized'],
  finalized: ['in_review', 'delivered'],
  delivered: [],
};

export const publicationStates: readonly PublicationState[] = ['draft', 'in_review', 'finalized', 'delivered'];

// 库里的 state 是自由字符串，历史数据可能读到没见过的值：认不出来时按原文显示、不给下一步，
// 而不是把它当草稿放开出编辑和跳级的口子。
export function isPublicationState(raw: string): raw is PublicationState {
  return publicationStates.includes(raw as PublicationState);
}

export function publicationStateLabel(raw: string) {
  return isPublicationState(raw) ? zh[`publicationState_${raw}`] : raw;
}

export function nextStateActions(raw: string): readonly PublicationState[] {
  return isPublicationState(raw) ? transitions[raw] : [];
}

export function publicationActionLabel(target: PublicationState) {
  return zh[`publicationStateAction_${target}`];
}
