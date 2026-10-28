import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { PublicationPanel } from './PublicationPanel';
import type { PublicationRepository } from '../../domain/publication';
afterEach(cleanup);

it('共享项目的出版物可读取，但不能创建或生成', async () => {
  const repository: PublicationRepository = {
    list: vi.fn().mockResolvedValue([{ id: 'book', projectId: 'project', pubType: 'photobook', title: '作品集', state: 'draft', createdAt: '2026-10-21' }]),
    create: vi.fn(), generate: vi.fn(), pages: vi.fn(),
  };
  render(<PublicationPanel repository={repository} projectId="project" sequenceCount={5} readOnly />);
  await screen.findByText('作品集', { selector: 'strong' });
  expect(screen.getByRole('textbox', { name: '出版物标题' })).toBeDisabled();
  expect(screen.getByRole('button', { name: '创建' })).toBeDisabled();
  expect(screen.getByRole('button', { name: /从序列生成/ })).toBeDisabled();
  expect(repository.create).not.toHaveBeenCalled();
  expect(repository.generate).not.toHaveBeenCalled();
});

it('创建失败保留输入并显示错误，不冒充成功或产生未处理 Promise', async () => {
  const repository: PublicationRepository = {
    list: vi.fn().mockResolvedValue([]),
    create: vi.fn().mockRejectedValue(new Error('项目已归档')),
    generate: vi.fn(), pages: vi.fn(),
  };
  const user = userEvent.setup();
  render(<PublicationPanel repository={repository} projectId="project" sequenceCount={5} />);
  await user.type(screen.getByRole('textbox', { name: '出版物标题' }), '光与停顿');
  await user.click(screen.getByRole('button', { name: '创建' }));
  expect(await screen.findByRole('status')).toHaveTextContent('项目已归档');
  expect(screen.getByRole('textbox', { name: '出版物标题' })).toHaveValue('光与停顿');
  expect(repository.create).toHaveBeenCalledExactlyOnceWith('project', '光与停顿');
});

it('生成后进入编辑器；已有页面须确认后才能重新编排', async () => {
  const publication={id:'book',projectId:'project',title:'摄影书',pubType:'photobook',state:'draft',revision:2,createdAt:'2026-10-25'};
  const document:import('../../domain/publication').PublicationDocument={revision:2,previews:[],layout:{schemaVersion:1,publication:{...publication,specJson:'{}'},pages:[{id:'page',pageNumber:1,side:'right',widthMm:190,heightMm:257,marginMm:15,bleedMm:3,layoutJson:null,frames:[],texts:[]}]}};
  const repository:PublicationRepository={list:vi.fn(),create:vi.fn(),generate:vi.fn(),pages:vi.fn(),document:vi.fn(),versions:vi.fn().mockResolvedValue([]),version:vi.fn()};
  let generated = false;
  repository.list = vi.fn().mockResolvedValue([publication]);
  repository.document = vi.fn().mockImplementation(async () => ({...document,layout:{...document.layout,pages:generated?document.layout.pages:[]}}));
  repository.generate = vi.fn().mockImplementation(async()=>{generated=true;return 1;});
  const user=userEvent.setup();
  render(<PublicationPanel repository={repository} projectId={publication.projectId} sequenceCount={1}/>);
  await user.click(await screen.findByRole('button',{name:'从序列生成（1 张）'}));
  expect(await screen.findByRole('dialog',{name:'版面与版本'})).toBeVisible();
  await user.click(screen.getByRole('button',{name:'关闭对话框'}));
  await user.click(screen.getByRole('button',{name:'从序列生成（1 张）'}));
  expect(await screen.findByRole('dialog',{name:'重新生成页面？'})).toBeVisible();
  expect(repository.generate).toHaveBeenCalledTimes(1);
  await user.click(screen.getByRole('button',{name:'重新生成'}));
  expect(await screen.findByRole('dialog',{name:'版面与版本'})).toBeVisible();
  expect(repository.generate).toHaveBeenLastCalledWith(publication.projectId,publication.id,1,true);
});

it('不同摄影书的每页照片数彼此独立，生成使用对应摄影书的选择',async()=>{
  const repository:PublicationRepository={list:vi.fn().mockResolvedValue(['A','B'].map(id=>({id,projectId:'p',pubType:'photobook',title:id,state:'draft',createdAt:'2026-10-25'}))),create:vi.fn(),generate:vi.fn().mockResolvedValue(1),pages:vi.fn()};
  const user=userEvent.setup();render(<PublicationPanel repository={repository} projectId="p" sequenceCount={5}/>);
  const a=await screen.findByRole('combobox',{name:'A · 每页照片数'}),b=screen.getByRole('combobox',{name:'B · 每页照片数'});
  await user.selectOptions(a,'2');await user.selectOptions(b,'4');expect(a).toHaveValue('2');expect(b).toHaveValue('4');
  await user.click(screen.getAllByRole('button',{name:'从序列生成（5 张）'})[0]);expect(repository.generate).toHaveBeenCalledWith('p','A',2,false);
});
