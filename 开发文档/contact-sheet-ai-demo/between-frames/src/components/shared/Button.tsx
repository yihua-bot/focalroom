import type { ButtonHTMLAttributes } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

// shadcn 风格基础控件；语义按钮不引入额外行为。
const variants = cva('button', { variants: { variant: { default: '', primary: 'primary', quiet: 'quiet' }, size: { default: '', small: 'tiny' } }, defaultVariants: { variant: 'default', size: 'default' } });
export function Button({ variant, size, className, ...props }: ButtonHTMLAttributes<HTMLButtonElement> & VariantProps<typeof variants>) {
  return <button type="button" className={twMerge(clsx(variants({ variant, size }), className))} {...props} />;
}
