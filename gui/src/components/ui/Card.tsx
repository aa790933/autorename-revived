/**
 * Base UI Components - Card, Badge, Separator
 */
import { HTMLAttributes, forwardRef } from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

// Card
interface CardProps extends HTMLAttributes<HTMLDivElement> {
  variant?: 'default' | 'bordered' | 'elevated';
  padding?: 'none' | 'sm' | 'md' | 'lg';
}

export const Card = forwardRef<HTMLDivElement, CardProps>(
  ({ className, variant = 'default', padding = 'md', children, ...props }, ref) => {
    const variants = {
      default: 'bg-white dark:bg-gray-800',
      bordered: 'bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700',
      elevated: 'bg-white dark:bg-gray-800 shadow-lg',
    };

    const paddings = {
      none: '',
      sm: 'p-3',
      md: 'p-5',
      lg: 'p-6',
    };

    return (
      <div
        ref={ref}
        className={twMerge('rounded-xl', variants[variant], paddings[padding], className)}
        {...props}
      >
        {children}
      </div>
    );
  }
);

Card.displayName = 'Card';

interface CardHeaderProps extends HTMLAttributes<HTMLDivElement> {}

export const CardHeader = forwardRef<HTMLDivElement, CardHeaderProps>(
  ({ className, children, ...props }, ref) => (
    <div
      ref={ref}
      className={twMerge('mb-4', className)}
      {...props}
    >
      {children}
    </div>
  )
);

CardHeader.displayName = 'CardHeader';

interface CardTitleProps extends HTMLAttributes<HTMLHeadingElement> {}

export const CardTitle = forwardRef<HTMLHeadingElement, CardTitleProps>(
  ({ className, children, ...props }, ref) => (
    <h3
      ref={ref}
      className={twMerge('text-lg font-semibold text-gray-900 dark:text-gray-100', className)}
      {...props}
    >
      {children}
    </h3>
  )
);

CardTitle.displayName = 'CardTitle';

interface CardDescriptionProps extends HTMLAttributes<HTMLParagraphElement> {}

export const CardDescription = forwardRef<HTMLParagraphElement, CardDescriptionProps>(
  ({ className, children, ...props }, ref) => (
    <p
      ref={ref}
      className={twMerge('text-sm text-gray-500 dark:text-gray-400 mt-1', className)}
      {...props}
    >
      {children}
    </p>
  )
);

CardDescription.displayName = 'CardDescription';

interface CardContentProps extends HTMLAttributes<HTMLDivElement> {}

export const CardContent = forwardRef<HTMLDivElement, CardContentProps>(
  ({ className, children, ...props }, ref) => (
    <div
      ref={ref}
      className={twMerge(className)}
      {...props}
    >
      {children}
    </div>
  )
);

CardContent.displayName = 'CardContent';

interface CardFooterProps extends HTMLAttributes<HTMLDivElement> {}

export const CardFooter = forwardRef<HTMLDivElement, CardFooterProps>(
  ({ className, children, ...props }, ref) => (
    <div
      ref={ref}
      className={twMerge('mt-4 pt-4 border-t border-gray-200 dark:border-gray-700 flex items-center gap-2', className)}
      {...props}
    >
      {children}
    </div>
  )
);

CardFooter.displayName = 'CardFooter';

// Badge
interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  variant?: 'default' | 'success' | 'warning' | 'danger' | 'info' | 'processing';
  size?: 'sm' | 'md';
}

export const Badge = forwardRef<HTMLSpanElement, BadgeProps>(
  ({ className, variant = 'default', size = 'md', children, ...props }, ref) => {
    const variants = {
      default: 'bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-300',
      success: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
      warning: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
      danger: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
      info: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
      processing: 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400 animate-pulse',
    };

    const sizes = {
      sm: 'px-2 py-0.5 text-xs',
      md: 'px-2.5 py-1 text-sm',
    };

    return (
      <span
        ref={ref}
        className={twMerge(
          'inline-flex items-center font-medium rounded-full',
          variants[variant],
          sizes[size],
          className
        )}
        {...props}
      >
        {children}
      </span>
    );
  }
);

Badge.displayName = 'Badge';

// Separator
interface SeparatorProps extends HTMLAttributes<HTMLHRElement> {
  orientation?: 'horizontal' | 'vertical';
}

export const Separator = forwardRef<HTMLHRElement, SeparatorProps>(
  ({ className, orientation = 'horizontal', ...props }, ref) => (
    <hr
      ref={ref}
      className={twMerge(
        'border-gray-200 dark:border-gray-700',
        orientation === 'horizontal' ? 'w-full' : 'h-full',
        className
      )}
      aria-orientation={orientation}
      {...props}
    />
  )
);

Separator.displayName = 'Separator';

// Label
interface LabelProps extends HTMLAttributes<HTMLLabelElement> {}

export const Label = forwardRef<HTMLLabelElement, LabelProps>(
  ({ className, children, ...props }, ref) => (
    <label
      ref={ref}
      className={twMerge('block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1', className)}
      {...props}
    >
      {children}
    </label>
  )
);

Label.displayName = 'Label';