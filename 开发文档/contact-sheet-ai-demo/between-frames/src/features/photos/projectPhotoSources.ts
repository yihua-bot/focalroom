import type { Photo } from '../../domain/models';
export type ProjectPhotoSource='sequence'|'all'|'selection';
/** 来源顺序与搜索相互独立；缺失编号不替换成另一张照片。 */
export function projectPhotoSources(photos:Photo[],sequence:string[],selection:string[]):Record<ProjectPhotoSource,Photo[]>{
  const byId=new Map(photos.map(photo=>[photo.id,photo]));
  const ordered=(ids:string[])=>[...new Set(ids)].flatMap(id=>byId.has(id)?[byId.get(id)!]:[]);
  return {sequence:ordered(sequence),all:ordered(photos.map(photo=>photo.id)),selection:ordered(selection)};
}
