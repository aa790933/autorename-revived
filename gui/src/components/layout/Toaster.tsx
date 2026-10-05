/**
 * Toast notifications
 */
import { useToastStore } from '@/hooks/useToast';
import { X, CheckCircle, AlertCircle, AlertTriangle, Info } from 'lucide-react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';
import { useTranslation } from 'react-i18next';

const ICONS = {
  success: CheckCircle,
  danger: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

const COLORS = {
  success: 'bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-800 text-green-800 dark:text-green-400',
  danger: 'bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800 text-red-800 dark:text-red-400',
  warning: 'bg-yellow-50 dark:bg-yellow-900/20 border-yellow-200 dark:border-yellow-800 text-yellow-800 dark:text-yellow-400',
  info: 'bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800 text-blue-800 dark:text-blue-400',
};

export function Toaster() {
  const { t } = useTranslation();
  const { toasts, removeToast } = useToastStore();
  const isRTL = document.documentElement.dir === 'rtl';

  if (toasts.length === 0) return null;

  return (
    <div
      className={clsx(
        'fixed bottom-4 z-50 flex flex-col gap-2 max-w-sm w-full pointer-events-none',
        isRTL ? 'left-4' : 'right-4'
      )}
      role="region"
      aria-label={t('common.info')}
      aria-live="polite"
    >
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={twMerge(
            'pointer-events-auto flex items-start gap-3 p-4 rounded-xl border shadow-lg animate-slide-in',
            COLORS[toast.type]
          )}
          role="alert"
        >
          <div className="flex-shrink-0 mt-0.5">
            {(() => {
              const IconComponent = ICONS[toast.type as keyof typeof ICONS];
              return <IconComponent className="w-5 h-5" aria-hidden="true" />;
            })()}
          </div>
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium">{toast.message}</p>
          </div>
          <button
            onClick={() => removeToast(toast.id)}
            className="flex-shrink-0 p-1 rounded hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
            aria-label={t('common.close')}
          >
            <X className="w-4 h-4 opacity-70 hover:opacity-100" />
          </button>
        </div>
      ))}
    </div>
  );
}