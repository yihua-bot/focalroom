import { render,screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect,it,vi } from 'vitest';
import { PublicationPages } from './PublicationPages';
import type { PublicationLayout } from '../../domain/publication';

it('长摄影书只渲染一批缩略图，当前页跳转会显示对应批次',async()=>{
  const pages:PublicationLayout['pages']=Array.from({length:85},(_,index)=>({id:`p${index}`,pageNumber:index,side:'right',widthMm:210,heightMm:297,marginMm:15,bleedMm:3,layoutJson:null,frames:[],texts:[]}));
  const onSelect=vi.fn(),user=userEvent.setup();const {rerender}=render(<PublicationPages pages={pages} previews={[]} activeId="p65" disabled={false} onSelect={onSelect}/>);
  expect(document.querySelectorAll('.publication-page-thumb')).toHaveLength(40);expect(screen.getByRole('button',{name:'第 65 页'})).toHaveAttribute('aria-current','page');
  await user.click(screen.getByRole('button',{name:'上一页'}));expect(screen.queryByRole('button',{name:'第 65 页'})).not.toBeInTheDocument();
  rerender(<PublicationPages pages={pages} previews={[]} activeId="p83" disabled={false} onSelect={onSelect}/>);expect(screen.getByRole('button',{name:'第 83 页'})).toHaveAttribute('aria-current','page');expect(document.querySelectorAll('.publication-page-thumb')).toHaveLength(5);
  await user.click(screen.getByRole('button',{name:'第 84 页'}));expect(onSelect).toHaveBeenCalledWith('p84');
});
