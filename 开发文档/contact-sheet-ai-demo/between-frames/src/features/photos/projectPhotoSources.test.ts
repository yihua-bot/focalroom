import { expect,it } from 'vitest';
import type { Photo } from '../../domain/models';
import { projectPhotoSources } from './projectPhotoSources';
it('来源只解析当前项目，保留作品顺序并去重，不用其他照片代替缺失编号',()=>{
  const photos=['a','b','c'].map((id,index)=>({id,index,name:id,src:id+'.jpg',rating:0,note:'',status:'undecided'})) as Photo[];
  const before=structuredClone(photos),sources=projectPhotoSources(photos,['c','foreign','a','c'],['b','missing','b']);expect(sources.sequence.map(photo=>photo.id)).toEqual(['c','a']);expect(sources.selection.map(photo=>photo.id)).toEqual(['b']);expect(sources.all).toEqual(photos);expect(photos).toEqual(before);
});
