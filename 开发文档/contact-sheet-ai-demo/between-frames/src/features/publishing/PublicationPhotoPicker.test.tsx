import { cleanup,render,screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,beforeEach,expect,it,vi } from 'vitest';
import type { Photo } from '../../domain/models';
import type { PublicationLayout } from '../../domain/publication';
import { setCopyLocale } from '../../app/copy';
import { PublicationPhotoPicker } from './PublicationPhotoPicker';
const photos:Photo[]=['A','B','C'].map((id,index)=>({id,index,name:id,src:id+'.jpg',status:'undecided',rating:0,note:''}));
const page:PublicationLayout['pages'][number]={id:'page',pageNumber:1,side:'right',widthMm:190,heightMm:257,marginMm:15,bleedMm:3,layoutJson:null,texts:[],frames:[{id:'old',xMm:15,yMm:15,widthMm:160,heightMm:227,fitMode:'fill',cropX:.2,cropY:.7,cropZoom:2,assetId:'different-asset',renditionId:'render',source:null}]};
const defaults={page,photos,previews:[{frameId:'old',photoId:'A',thumbnailSrc:'checked-render.jpg'}],sequencePhotoIds:['C','A'],selectedPhotoIds:['B'],busy:false,pending:false,error:''};
beforeEach(()=>setCopyLocale('zh'));afterEach(cleanup);
it('本页照片禁选，先显示排版预览，确认后按来源顺序保存',async()=>{
  const onSave=vi.fn().mockResolvedValue(true),onClose=vi.fn();render(<PublicationPhotoPicker {...defaults} onSave={onSave} onClose={onClose}/>);const user=userEvent.setup();
  expect(screen.getByRole('checkbox',{name:'A本页已有'})).toBeDisabled();await user.click(screen.getByRole('checkbox',{name:'C'}));expect(onSave).not.toHaveBeenCalled();expect(document.querySelector('.project-photo-picker-preview img[src="C.jpg"]')).not.toBeNull();expect(document.querySelector('.project-photo-picker-preview img[src="checked-render.jpg"]')).not.toBeNull();
  await user.click(screen.getByRole('button',{name:'排入本页（1 张）'}));expect(onSave).toHaveBeenCalledExactlyOnceWith(['C'],0);expect(onClose).toHaveBeenCalledOnce();
});
it('替换只选一张，预览重置裁切，原位置与尺寸保持不动',async()=>{
  const onSave=vi.fn().mockResolvedValue(true);render(<PublicationPhotoPicker {...defaults} sequencePhotoIds={[]} replacement="old" onSave={onSave} onClose={vi.fn()}/>);const user=userEvent.setup();await user.click(screen.getByRole('radio',{name:'C'}));await user.click(screen.getByRole('radio',{name:'B'}));expect(screen.getByRole('radio',{name:'C'})).not.toBeChecked();
  const image=document.querySelector('.project-photo-picker-preview img[src="B.jpg"]') as HTMLElement;expect(image).toHaveStyle({transform:'scale(1)'});expect(page.frames[0].cropZoom).toBe(2);await user.click(screen.getByRole('button',{name:'替换照片'}));expect(onSave).toHaveBeenCalledExactlyOnceWith(['B'],0);
});
it('结果待确认时冻结选图与再次保存，但可以返回确认原请求',async()=>{
  const onClose=vi.fn();render(<PublicationPhotoPicker {...defaults} pending onClose={onClose} onSave={vi.fn()}/>);expect(screen.getByRole('checkbox',{name:'C'})).toBeDisabled();expect(screen.getByRole('button',{name:'排入本页（0 张）'})).toBeDisabled();await userEvent.setup().click(screen.getByRole('button',{name:'取消'}));expect(onClose).toHaveBeenCalledOnce();
});
