import { cleanup,render,screen,within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,beforeEach,expect,it,vi } from 'vitest';
import type { PublicationLayout } from '../../domain/publication';
import { setCopyLocale } from '../../app/copy';
import { PublicationPageEditor,PublicationTextEditor } from './PublicationEditDialogs';
import { PublicationPaper } from './PublicationPaper';
const page:PublicationLayout['pages'][number]={id:crypto.randomUUID(),pageNumber:2,side:'left',widthMm:190,heightMm:257,marginMm:15,bleedMm:3,layoutJson:null,frames:[],texts:[]};
beforeEach(()=>setCopyLocale('zh'));afterEach(cleanup);
it('页面尺寸先检查现有内容范围，未保存关闭需要明确放弃',async()=>{
  const onSave=vi.fn().mockResolvedValue(true),onClose=vi.fn();
  render(<PublicationPageEditor page={{...page,frames:[{id:'f',xMm:15,yMm:15,widthMm:150,heightMm:100,fitMode:'fit',cropX:.5,cropY:.5,cropZoom:1,assetId:null,renditionId:null,source:null}]}} isNew={false} busy={false} pending={false} notice="" onClose={onClose} onSave={onSave}/>);
  const user=userEvent.setup();const width=screen.getByRole('spinbutton',{name:'页面宽度（mm）'});await user.clear(width);await user.type(width,'50');
  expect(screen.getByRole('alert')).toHaveTextContent('已有内容');expect(screen.getByRole('button',{name:'保存调整'})).toBeDisabled();expect(onSave).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button',{name:'关闭对话框'}));expect(onClose).not.toHaveBeenCalled();
  await user.click(within(screen.getByRole('group',{name:'放弃尚未保存的调整？'})).getByRole('button',{name:'放弃调整'}));expect(onClose).toHaveBeenCalledOnce();
});
it('文字先预览，HTML原样显示且不会执行；保存后才关闭',async()=>{
  const onSave=vi.fn().mockResolvedValue(true),onClose=vi.fn(),id=crypto.randomUUID();
  render(<PublicationTextEditor page={page} textId={id} kind="text" busy={false} pending={false} notice="" previews={[]} onClose={onClose} onSave={onSave}/>);
  const user=userEvent.setup();await user.type(screen.getByRole('textbox',{name:'文字内容'}),'<script>hello</script>');
  expect(document.querySelector('.publication-object-preview')?.textContent).toContain('<script>hello</script>');expect(document.querySelector('.publication-object-preview script')).toBeNull();expect(onSave).not.toHaveBeenCalled();
  await user.click(screen.getByRole('checkbox',{name:'加粗'}));await user.selectOptions(screen.getByRole('combobox',{name:'对齐'}),'center');
  await user.click(screen.getByRole('button',{name:'保存调整'}));expect(onSave).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({textId:id,pageId:page.id,text:'<script>hello</script>',bold:true,align:'center'}));expect(onClose).toHaveBeenCalledOnce();
});
it('错误或待确认保存保留编辑内容，不把失败当保存成功',async()=>{
  const onSave=vi.fn().mockResolvedValue(false),onClose=vi.fn();
  const props={page,textId:crypto.randomUUID(),kind:'text' as const,busy:false,pending:false,notice:'',previews:[],onClose,onSave};
  const view=render(<PublicationTextEditor {...props}/>);const user=userEvent.setup();await user.type(screen.getByRole('textbox',{name:'文字内容'}),'保留这句话');await user.click(screen.getByRole('button',{name:'保存调整'}));expect(onClose).not.toHaveBeenCalled();
  view.rerender(<PublicationTextEditor {...props} pending notice="请确认上一次保存"/>);expect(screen.getByRole('textbox',{name:'文字内容'})).toHaveValue('保留这句话');expect(screen.getByRole('button',{name:'保存调整'})).toBeDisabled();expect(screen.getByRole('status')).toHaveTextContent('请确认');expect(screen.getByRole('button',{name:'返回'})).toBeEnabled();expect(screen.queryByRole('button',{name:'放弃调整'})).not.toBeInTheDocument();
});
it('移除页面先解释连带内容，明确确认后才请求',async()=>{
  const onRemove=vi.fn().mockResolvedValue(true),onClose=vi.fn();
  render(<PublicationPageEditor page={page} isNew={false} busy={false} pending={false} notice="" onClose={onClose} onSave={vi.fn()} onRemove={onRemove}/>);const user=userEvent.setup();
  await user.click(screen.getByRole('button',{name:'移除页面'}));expect(onRemove).not.toHaveBeenCalled();expect(screen.getByText(/照片框和文字会一并移除/)).toBeVisible();await user.click(screen.getByRole('button',{name:'确认移除页面'}));expect(onRemove).toHaveBeenCalledOnce();expect(onClose).toHaveBeenCalledOnce();
});
it('页码使用当前页序渲染，封面不显示；缩略图没有嵌套按钮',()=>{
  const text={id:'page-number',xMm:15,yMm:230,widthMm:100,heightMm:10,contentJson:JSON.stringify({kind:'page_number',text:'99'}),styleJson:JSON.stringify({fontSizePt:10,fontFamily:'monospace',color:'#123456'})};
  const view=render(<PublicationPaper page={{...page,texts:[text]}} previews={[]} label="版面" thumbnail/>);
  expect(document.querySelector('.publication-text')).toHaveTextContent('2');expect(document.querySelector('button')).toBeNull();
  view.rerender(<PublicationPaper page={{...page,pageNumber:7,texts:[text]}} previews={[]} label="版面" thumbnail/>);expect(document.querySelector('.publication-text')).toHaveTextContent('7');
  view.rerender(<PublicationPaper page={{...page,pageNumber:0,texts:[text]}} previews={[]} label="版面" thumbnail/>);expect(document.querySelector('.publication-text')).toHaveTextContent('');
});

it('新增页面和文字编辑在英语、日语和韩语中不显示内部文案键',()=>{
  for(const [locale,pageTitle,textTitle] of [['en','Add page','Add text'],['ja','ページを追加','文字を追加'],['ko','페이지 추가','텍스트 추가']]){
    setCopyLocale(locale);render(<PublicationPageEditor page={page} isNew busy={false} pending={false} notice="" onClose={vi.fn()} onSave={vi.fn()}/>);
    expect(screen.getByRole('dialog',{name:pageTitle})).toBeVisible();expect(document.body.textContent).not.toMatch(/publication[A-Z]/);cleanup();
    render(<PublicationTextEditor page={page} textId="text" kind="text" busy={false} pending={false} notice="" previews={[]} onClose={vi.fn()} onSave={vi.fn()}/>);
    expect(screen.getByRole('dialog',{name:textTitle})).toBeVisible();expect(document.body.textContent).not.toMatch(/publication[A-Z]/);cleanup();
  }
});
