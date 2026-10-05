/**
 * Toggle / switch component
 */

import { forwardRef, InputHTMLAttributes, HTMLAttributes } from 'react';
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
      <label className={twMerge('flex items-start gap-3 cursor-pointer', className)}>
        <div className="relative mt-1">
          <input
            ref={ref}
            type="checkbox"
            id={inputId}
            className="peer h-6 w-6 appearance-none rounded-full border-2 border-neutral-300 dark:border-neutral-600 bg-white dark:bg-neutral-800 transition-all duration-200
              checked:bg-primary-600 checked:border-primary-600
              checked:after:translate-x-full
              after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:h-4 after:w-4 after:bg-white after:rounded-full after:transition-transform after:duration-200
              focus:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-neutral-950
              disabled:opacity-50 disabled:cursor-not-allowed"
            {...props}
          />
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
interface RadioGroupProps extends Omit<HTMLAttributes<HTMLDivElement>, 'onChange'> {
  name: string;
  value: string;
  onChange: (value: string) => void;
  children: React.ReactNode;
}

export function RadioGroup({ name, value, onChange, children, className, ...props }: RadioGroupProps) {
  return (
    <div role="radiogroup" aria-label={name} className={twMerge('space-y-2', className)} {...props}>
      {children}
    </div>
  );
}

interface RadioProps extends InputHTMLAttributes<HTMLInputElement> {
  label: string;
  value: string;
  description?: string;
}

export const Radio = forwardRef<HTMLInputElement, RadioProps>(
  ({ className, label, value, description, id, ...props }, ref) => {
    const inputId = id || `radio-${value}`;

    return (
      <label className={twMerge('flex items-start gap-3 cursor-pointer', className)}>
        <input
          ref={ref}
          type="radio"
          id={inputId}
          value={value}
          className="peer h-4 w-4 appearance-none border-2 border-neutral-300 text-primary-600 rounded-full transition-all duration-200
            checked:border-primary-600
            checked:after:absolute checked:after:top-1/2 checked:after:left-1/2 checked:after:-translate-y-1/2 checked:after:-translate-x-1/2 checked:after:h-2 checked:after:w-2 checked:after:bg-primary-600 checked:after:rounded-full
            focus:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-2 dark:border-neutral-600 dark:focus-visible:ring-offset-neutral-950
            disabled:opacity-50 disabled:cursor-not-allowed"
          {...props}
        />
        <div className="flex flex-col mt-1">
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