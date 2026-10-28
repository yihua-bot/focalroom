import { cleanup,fireEvent,render,screen } from '@testing-library/react';
import { afterEach,expect,it,vi } from 'vitest';
import { PublicationDimensionInput } from './PublicationDimensionInput';
afterEach(cleanup);
it('显示简化小数不产生修改，聚焦保留完整精度，用户输入才回传',()=>{
  const change=vi.fn();render(<PublicationDimensionInput aria-label="宽度" value="97.08474576271186" onChange={change}/>);const input=screen.getByRole('spinbutton',{name:'宽度'});expect(input).toHaveValue(97.085);fireEvent.focus(input);expect(input).toHaveValue(97.08474576271186);fireEvent.blur(input);expect(input).toHaveValue(97.085);expect(change).not.toHaveBeenCalled();fireEvent.change(input,{target:{value:'81.12345'}});expect(change).toHaveBeenCalledExactlyOnceWith('81.12345');
});
it('微小正数不会被显示成零，空白值保持可编辑',()=>{const props={value:'0.0001',onChange:vi.fn()};const view=render(<PublicationDimensionInput {...props}/>);expect(screen.getByRole('spinbutton')).toHaveValue(.0001);view.rerender(<PublicationDimensionInput {...props} value=""/>);expect(screen.getByRole('spinbutton')).toHaveValue(null);});
