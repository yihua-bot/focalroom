import { cleanup,render,screen,waitFor,fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,beforeEach,expect,it,vi } from 'vitest';
import type { PublicationLayoutFrame,PublicationPhotoVersion,PublicationPhotoVersions,PublicationRepository } from '../../domain/publication';
import { setCopyLocale } from '../../app/copy';
import { PublicationVersionPicker } from './PublicationVersionPicker';
const source={assetId:'asset',sourceSha256:'a'.repeat(64),width:4000,height:3000,sourceProfileDigest:null,renditionId:null,variantId:null,renditionSha256:null,renditionWidth:null,renditionHeight:null,bitDepth:null,iccDigest:null,previewSha256:'c'.repeat(64)};
const frame:PublicationLayoutFrame={id:'frame',xMm:15,yMm:15,widthMm:100,heightMm:150,fitMode:'fill',cropX:.4,cropY:.6,cropZoom:1.5,assetId:'asset',renditionId:null,source};
const original:PublicationPhotoVersion={renditionId:null,variantName:null,externalSoftware:null,createdAt:null,source,thumbnailSrc:'original.jpg'};
const edited:PublicationPhotoVersion={...original,renditionId:'version',variantName:'冷调',externalSoftware:'lightroom_classic',source:{...source,renditionId:'version',renditionWidth:2000,renditionHeight:1500},thumbnailSrc:'blue.jpg'};
const result:PublicationPhotoVersions={revision:2,frameId:'frame',assetId:'asset',original,current:original,choices:[edited],next:null};
function props(photoVersions=vi.fn().mockResolvedValue(result)) {return {repository:{list:vi.fn(),create:vi.fn(),generate:vi.fn(),pages:vi.fn(),photoVersions} as PublicationRepository,projectId:'project',publicationId:'book',revision:2,frame,busy:false,pending:false,notice:'',onSave:vi.fn().mockResolvedValue(true),onClose:vi.fn()};}
beforeEach(()=>setCopyLocale('zh'));afterEach(()=>cleanup());
it('原片和修图预览可比较，仅在明确确认后提交选定版本',async()=>{
  const p=props();render(<PublicationVersionPicker {...p}/>);const user=userEvent.setup();await screen.findByRole('radio',{name:/冷调/});expect(p.onSave).not.toHaveBeenCalled();expect(screen.getByRole('button',{name:'使用此版本'})).toBeDisabled();await user.click(screen.getByRole('radio',{name:/冷调/}));expect(screen.getByRole('img',{name:'当前使用'})).toHaveAttribute('src','original.jpg');expect(screen.getByRole('img',{name:'预览版本'})).toHaveAttribute('src','blue.jpg');expect(screen.getByRole('button',{name:'使用此版本'})).toBeDisabled();fireEvent.load(screen.getByRole('img',{name:'预览版本'}));await user.click(screen.getByRole('button',{name:'使用此版本'}));expect(p.onSave).toHaveBeenCalledExactlyOnceWith('version');expect(p.onClose).toHaveBeenCalledOnce();
});
it('没有预览的版本不可选，结果不明时禁止新保存但可返回确认原请求',async()=>{
  const missing={...edited,thumbnailSrc:undefined};const p=props(vi.fn().mockResolvedValue({...result,choices:[missing]}));const view=render(<PublicationVersionPicker {...p}/>);await screen.findByRole('radio',{name:/冷调/});expect(screen.getByRole('radio',{name:/冷调/})).toBeDisabled();view.rerender(<PublicationVersionPicker {...p} pending/>);expect(screen.getByRole('button',{name:'使用此版本'})).toBeDisabled();await userEvent.click(screen.getByRole('button',{name:'返回'}));expect(p.onClose).toHaveBeenCalledOnce();expect(p.onSave).not.toHaveBeenCalled();
});
it('项目切换立即清除旧目录，迟到旧请求不能恢复旧预览',async()=>{
  let finish!:(value:PublicationPhotoVersions)=>void;const load=vi.fn().mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;})).mockResolvedValue({...result,choices:[]});const p=props(load);const view=render(<PublicationVersionPicker {...p}/>);await waitFor(()=>expect(load).toHaveBeenCalledOnce());view.rerender(<PublicationVersionPicker {...p} projectId="other"/>);await screen.findByText('还没有修图版本。修图结果回传后可在这里选择。');finish(result);await waitFor(()=>expect(screen.queryByRole('radio',{name:/冷调/})).toBeNull());expect(p.onSave).not.toHaveBeenCalled();
});

it('浏览器无法加载图片时显示不可用并阻止确认，不靠有效地址冒充成功显示',async()=>{const p=props();render(<PublicationVersionPicker {...p}/>);await screen.findByRole('radio',{name:/冷调/});await userEvent.click(screen.getByRole('radio',{name:/冷调/}));fireEvent.error(screen.getByRole('img',{name:'预览版本'}));expect(screen.queryByRole('img',{name:'预览版本'})).toBeNull();expect(screen.getByRole('radio',{name:/冷调/})).toBeDisabled();expect(screen.getByRole('button',{name:'使用此版本'})).toBeDisabled();expect(p.onSave).not.toHaveBeenCalled();});
