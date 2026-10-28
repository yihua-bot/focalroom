import { useState,type InputHTMLAttributes } from 'react';
/** 阅读时简化小数，编辑与保存仍使用完整数值；格式化本身不修改版面。 */
export function PublicationDimensionInput({value,onChange,...props}:Omit<InputHTMLAttributes<HTMLInputElement>,'value'|'onChange'|'type'>&{value:string;onChange:(value:string)=>void}){
  const [focused,setFocused]=useState(false);const number=Number(value);
  const shown=focused||!value.trim()||!Number.isFinite(number)||number!==0&&Math.abs(number)<.001?value:String(Math.round(number*1000)/1000);
  return <input {...props} type="number" value={shown} onFocus={event=>{setFocused(true);props.onFocus?.(event);}} onBlur={event=>{setFocused(false);props.onBlur?.(event);}} onChange={event=>onChange(event.target.value)}/>;
}
