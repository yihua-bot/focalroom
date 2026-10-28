import { cleanup,render,screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,beforeEach,expect,it,vi } from 'vitest';
import { PublicationMaterials } from './PublicationMaterials';
import { setCopyLocale } from '../../app/copy';
import type { Photo } from '../../domain/models';
const photos=Array.from({length:55},(_,index)=>({id:'photo-'+index,index,name:'PHOTO_'+index,src:'photo.jpg',rating:0,status:'undecided',note:''})) as Photo[];
const props={photos,sequencePhotoIds:['photo-4','photo-1'],selectedPhotoIds:['photo-30'],placedPhotoIds:['photo-4'],disabled:false,onPlace:vi.fn(),onDragStart:vi.fn(),onDragEnd:vi.fn()};
beforeEach(()=>{setCopyLocale('zh');vi.clearAllMocks();});afterEach(cleanup);
it('默认按作品序列，已在本页不可重复拖入；键盘放置与拖动共用同一照片身份',async()=>{
  render(<PublicationMaterials {...props}/>);const user=userEvent.setup();expect(screen.getByRole('button',{name:'放置 PHOTO_4'})).toBeDisabled();expect(screen.getByRole('button',{name:'放置 PHOTO_4'})).toHaveAttribute('draggable','false');await user.click(screen.getByRole('button',{name:'放置 PHOTO_1'}));expect(props.onPlace).toHaveBeenCalledExactlyOnceWith(photos[1]);expect(props.onDragStart).not.toHaveBeenCalled();
});
it('所有来源和搜索分页有界，只读冻结放置且无开发翻译键',async()=>{
  const view=render(<PublicationMaterials {...props}/>);const user=userEvent.setup();await user.selectOptions(screen.getByRole('combobox',{name:'照片来源'}),'all');expect(screen.getAllByRole('button',{name:/^放置 PHOTO_/})).toHaveLength(24);await user.click(screen.getByRole('button',{name:'下一页'}));expect(screen.getByRole('button',{name:'放置 PHOTO_30'})).toBeVisible();await user.type(screen.getByRole('searchbox'),'PHOTO_54');expect(screen.getAllByRole('button',{name:/^放置 PHOTO_/})).toHaveLength(1);view.rerender(<PublicationMaterials {...props} disabled/>);expect(screen.getByRole('button',{name:'放置 PHOTO_54'})).toBeDisabled();expect(screen.getByRole('searchbox')).toBeDisabled();expect(document.body.textContent).not.toMatch(/publication[A-Z]/);
});
