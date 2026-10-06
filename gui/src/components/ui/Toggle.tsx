/**
 * Toggle / switch component
 */

import { forwardRef, InputHTMLAttributes } from 'react';
import { twMerge } from 'tailwind-merge';

// Toggle / Switch
interface ToggleProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  description?: string;
}

export const Toggle = forwardRef<HTMLInputElement, ToggleProps>(
  ({ className, label, description, id, ...props }, ref) => {
    const inputId = id || label?.toLowerCase().replace(/\s+/g, '-');

    return (
      <label className={twMerge('group flex items-start gap-3 cursor-pointer select-none', className)}>
        <div className="relative mt-1">
          <input
            ref={ref}
            type="checkbox"
            id={inputId}
            className="peer sr-only"
            {...props}
          />
          <div className="h-5 w-9 rounded-full bg-neutral-200 dark:bg-neutral-700 transition-colors duration-200 peer-checked:bg-primary-600 peer-focus-visible:ring-2 peer-focus-visible:ring-primary-500 peer-focus-visible:ring-offset-2 dark:peer-focus-visible:ring-offset-neutral-950 peer-disabled:opacity-50" />
          <div className="absolute top-1 left-1 h-3 w-3 rounded-full bg-white transition-transform duration-200 peer-checked:translate-x-4" />
        </div>
        <div className="flex flex-col">
          {label && (
            <span className="text-sm font-medium text-neutral-900 dark:text-neutral-100">
              {label}
            </span>
          )}
          {description && (
            <span className="text-xs text-neutral-500 dark:text-neutral-400">
              {description}
            </span>
          )}
        </div>
      </label>
    );
  }
);

Toggle.displayName = 'Toggle';

// Checkbox
interface CheckboxProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
}

export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(
  ({ className, label, id, ...props }, ref) => {
    const inputId = id || label?.toLowerCase().replace(/\s+/g, '-');

    return (
      <label className={twMerge('flex items-center gap-2 cursor-pointer', className)}>
        <input
          ref={ref}
          type="checkbox"
          id={inputId}
          className="h-4 w-4 rounded border-neutral-300 text-primary-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-2 dark:border-neutral-600 dark:bg-neutral-800 dark:focus-visible:ring-offset-neutral-950 disabled:opacity-50 disabled:cursor-not-allowed"
          {...props}
        />
        {label && (
          <span className="text-sm text-neutral-700 dark:text-neutral-300">{label}</span>
        )}
      </label>
    );
  }
);

Checkbox.displayName = 'Checkbox';

// Radio Group
interface RadioGroupProps extends React.HTMLAttributes<HTMLDivElement> {
  label?: string;
}

export function RadioGroup({ label, children, className, ...props }: RadioGroupProps) {
  return (
    <div role="radiogroup" className={twMerge('space-y-3', className)} {...props}>
      {label && <span className="text-sm font-medium text-neutral-700 dark:text-neutral-300 mb-2 block">{label}</span>}
      {children}
    </div>
  );
}

interface RadioProps extends InputHTMLAttributes<HTMLInputElement> {
  label: string;
  description?: string;
}

export const Radio = forwardRef<HTMLInputElement, RadioProps>(
  ({ className, label, description, id, ...props }, ref) => {
    const inputId = id || `radio-${props.value}`;

    return (
      <label className={twMerge('flex items-start gap-3 cursor-pointer select-none', className)}>
        <div className="relative mt-1 flex items-center justify-center">
          <input
            ref={ref}
            type="radio"
            id={inputId}
            className="peer h-4 w-4 appearance-none border-2 border-neutral-300 dark:border-neutral-600 rounded-full transition-all duration-200
              checked:border-primary-600 checked:bg-primary-600
              focus:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-neutral-950
              disabled:opacity-50 disabled:cursor-not-allowed"
            {...props}
          />
          <div className="absolute h-1.5 w-1.5 rounded-full bg-white opacity-0 peer-checked:opacity-100 transition-opacity duration-200" />
        </div>
        <div className="flex flex-col">
          <span className="text-sm font-medium text-neutral-900 dark:text-neutral-100">{label}</span>
          {description && (
            <span className="text-xs text-neutral-500 dark:text-neutral-400">{description}</span>
          )}
        </div>
      </label>
    );
  }
);

Radio.displayName = 'Radio';