import { cleanup,render,screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,beforeEach,expect,it,vi } from 'vitest';
import { continuousFixture } from '../../test/publicationContinuousFixture';
import { setCopyLocale,zh } from '../../app/copy';
import { PublicationTextEditor } from './PublicationEditDialogs';
beforeEach(()=>setCopyLocale('zh'));afterEach(()=>{cleanup();setCopyLocale('zh');});
function props(){const {layout}=continuousFixture();return {page:layout.pages[1],textId:crypto.randomUUID(),kind:'text' as const,busy:false,pending:false,notice:'',previews:[],onClose:vi.fn(),onSave:vi.fn().mockResolvedValue(true)};}
it('切换方向先改变框内预览，确认保存才提交方向，物理位置尺寸不交换',async()=>{
 const value=props(),user=userEvent.setup();render(<PublicationTextEditor {...value}/>);await user.type(screen.getByRole('textbox',{name:'文字内容'}),'Night Walk');await user.selectOptions(screen.getByRole('combobox',{name:'文字方向'}),'rotate-270');
 expect(document.querySelector('.publication-text-content')).toHaveAttribute('data-text-direction','rotate-270');expect(document.querySelector('.publication-text-content')).toHaveStyle({transform:'translate(-50%, -50%) rotate(270deg)'});expect(value.onSave).not.toHaveBeenCalled();
 await user.click(screen.getByRole('button',{name:'保存调整'}));expect(value.onSave).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({textDirection:'rotate-270',pageId:value.page.id,xMm:15,yMm:15,widthMm:160,heightMm:35,text:'Night Walk'}));expect(value.onClose).toHaveBeenCalledOnce();
});
it('旧文字未操作方向时不补字段，待确认时方向被冻结',async()=>{
 const value=props(),user=userEvent.setup(),view=render(<PublicationTextEditor {...value}/>);await user.type(screen.getByRole('textbox',{name:'文字内容'}),'只改文字');expect(screen.getByRole('combobox',{name:'文字方向'})).toHaveValue('horizontal');await user.click(screen.getByRole('button',{name:'保存调整'}));expect(value.onSave.mock.calls[0][0]).not.toHaveProperty('textDirection');
 view.rerender(<PublicationTextEditor {...value} pending notice="等待确认"/>);expect(screen.getByRole('combobox',{name:'文字方向'})).toBeDisabled();expect(screen.getByRole('button',{name:'保存调整'})).toBeDisabled();
});
it('四语方向选项完整，竖排预览保留原文，取消不保存',async()=>{
 for(const locale of ['zh','en','ja','ko']){setCopyLocale(locale);const value=props(),user=userEvent.setup();render(<PublicationTextEditor {...value}/>);const select=screen.getByRole('combobox',{name:zh.publicationTextDirection});expect(select.querySelectorAll('option')).toHaveLength(5);await user.type(screen.getByRole('textbox',{name:zh.publicationTextContent}),'夜行');await user.selectOptions(select,'vertical-rl');expect(document.querySelector('.publication-text-content')).toHaveStyle({writingMode:'vertical-rl'});expect(document.querySelector('.publication-text-content')).toHaveTextContent('夜行');await user.click(screen.getByRole('button',{name:zh.publicationDiscardChanges}));expect(value.onClose).toHaveBeenCalledOnce();expect(value.onSave).not.toHaveBeenCalled();cleanup();}
});
