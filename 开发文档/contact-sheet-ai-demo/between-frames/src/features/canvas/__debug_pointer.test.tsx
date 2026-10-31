import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, it, vi } from 'vitest';
import { NarrativeCanvas } from './NarrativeCanvas';
import type { CanvasRepository } from '../../domain/canvas';

beforeEach(() => {
  Element.prototype.setPointerCapture ??= () => {};
  Element.prototype.releasePointerCapture ??= () => {};
});

it('debug pointer', async () => {
  const repo: CanvasRepository = {
    listBoards: vi.fn().mockResolvedValue([{ id: 'board-1', projectId: 'project', title: '第一章', state: 'draft' }]),
    createBoard: vi.fn(), getNodes: vi.fn().mockResolvedValue([]), getAnnotations: vi.fn().mockResolvedValue([]),
    placePhoto: vi.fn(), moveNodes: vi.fn(), addText: vi.fn(), setNodeText: vi.fn(), deleteNode: vi.fn(),
    addAnnotation: vi.fn(), resolveAnnotation: vi.fn(), addArrow: vi.fn(), undo: vi.fn(), redo: vi.fn(), adoptBoard: vi.fn(), addInk: vi.fn(), groupNodes: vi.fn(), resizeNode: vi.fn(), setNodeLocked: vi.fn(),
  };
  const user = userEvent.setup();
  const { container } = render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  await waitFor(() => expect(screen.getByRole('button', { name: '便签' })).toBeEnabled());
  const surface = container.querySelector('.canvas-surface')!;
  const seen: string[] = [];
  document.addEventListener('pointerdown', e => seen.push(`down ${e.target === surface ? 'surface' : 'other'} btn=${e.button}`), true);
  document.addEventListener('pointerup', e => seen.push(`up ${e.target === surface ? 'surface' : 'other'}`), true);
  document.addEventListener('mousedown', e => seen.push(`md ${e.target === surface ? 'surface' : 'other'}`), true);
  document.addEventListener('click', e => seen.push(`click ${e.target === surface ? 'surface' : 'other'}`), true);
  await user.click(screen.getByRole('button', { name: '便签' }));
  console.log('after toolbar:', seen.join(' | '));
  seen.length = 0;
  await user.pointer({ keys: '[MouseLeftDown]', target: surface, coords: { clientX: 30, clientY: 40 } });
  console.log('after down:', seen.join(' | '), 'textbox=', !!screen.queryByRole('textbox', { name: '便签内容' }));
  await user.pointer({ keys: '[MouseLeftUp]', target: surface });
  console.log('after up:', seen.join(' | '), 'textbox=', !!screen.queryByRole('textbox', { name: '便签内容' }), 'surface=', surface.className);
  expect(true).toBe(false);
});
