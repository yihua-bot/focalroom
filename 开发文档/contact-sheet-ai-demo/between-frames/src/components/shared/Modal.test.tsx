import { cleanup,render,screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,expect,it,vi } from 'vitest';
import { Modal } from './Modal';
afterEach(cleanup);
it('内层手势可阻止Esc关闭窗口，但事件仍能取消手势；普通Esc继续关闭',async()=>{
  const close=vi.fn(),cancel=vi.fn(),user=userEvent.setup();document.addEventListener('keydown',cancel);
  try{const view=render(<Modal title="创作窗口" onClose={close} onEscapeKeyDown={event=>event.preventDefault()}><button>页面</button></Modal>);await user.keyboard('{Escape}');expect(close).not.toHaveBeenCalled();expect(cancel).toHaveBeenCalled();expect(screen.getByRole('dialog',{name:'创作窗口'})).toBeVisible();view.rerender(<Modal title="创作窗口" onClose={close}><button>页面</button></Modal>);await user.keyboard('{Escape}');expect(close).toHaveBeenCalledOnce();}finally{document.removeEventListener('keydown',cancel);}
});
