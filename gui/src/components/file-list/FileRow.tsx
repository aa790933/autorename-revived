// Individual file row
import { useTranslation } from 'react-i18next';
import { ChevronRight, CheckCircle, AlertCircle, Clock, XCircle, Loader2, AlertTriangle } from 'lucide-react';
import { Button, Badge } from '@/components/ui';
import { clsx } from 'clsx';
import { escapeHtml, truncate } from '@/utils/helpers';
import type { FileEntry } from '@/types';

interface FileRowProps {
  file: FileEntry;
  onSelectSuggestion: (fileId: string, suggestionName: string) => void;
}

const STATUS_CONFIG = {
  pending: { label: 'files.status.pending', icon: Clock, variant: 'default' as const },
  processing: { label: 'files.status.processing', icon: Loader2, variant: 'processing' as const },
  completed: { label: 'files.status.completed', icon: CheckCircle, variant: 'success' as const },
  skipped: { label: 'files.status.skipped', icon: XCircle, variant: 'warning' as const },
  failed: { label: 'files.status.failed', icon: AlertCircle, variant: 'danger' as const },
};

export function FileRow({ file, onSelectSuggestion }: FileRowProps) {
  const { t } = useTranslation();
  const statusConfig = STATUS_CONFIG[file.status];
  const isPreview = file.status === 'pending' && file.result?.new_name;
  const newName = file.result?.new_name;
  const error = file.result?.error;
  const warnings = file.result?.warnings ?? [];
  const suggestions = file.result?.suggestion_names ?? [];
  const suggestionLanguages = file.result?.suggestion_languages ?? [];

  const StatusIcon = statusConfig.icon;

  return (
    <div
      className={clsx(
        'group flex items-start gap-3 p-4 rounded-xl border transition-all duration-200',
        'bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700',
        file.status === 'processing' && 'ring-2 ring-blue-500/50',
        file.status === 'completed' && 'border-green-200 dark:border-green-800',
        file.status === 'failed' && 'border-red-200 dark:border-red-800'
      )}
      data-file-id={file.id}
    >
      {/* Status indicator */}
      <div className="flex-shrink-0 flex items-center justify-center w-10 h-10">
        <div className={clsx(
          'w-8 h-8 rounded-full flex items-center justify-center',
          file.status === 'pending' && 'bg-gray-100 dark:bg-gray-700',
          file.status === 'processing' && 'bg-blue-100 dark:bg-blue-900/30 animate-pulse',
          file.status === 'completed' && 'bg-green-100 dark:bg-green-900/30',
          file.status === 'skipped' && 'bg-yellow-100 dark:bg-yellow-900/30',
          file.status === 'failed' && 'bg-red-100 dark:bg-red-900/30'
        )}>
          <StatusIcon className={clsx(
            'w-4 h-4',
            file.status === 'pending' && 'text-gray-400',
            file.status === 'processing' && 'text-blue-600 dark:text-blue-400 animate-spin',
            file.status === 'completed' && 'text-green-600 dark:text-green-400',
            file.status === 'skipped' && 'text-yellow-600 dark:text-yellow-400',
            file.status === 'failed' && 'text-red-600 dark:text-red-400'
          )} aria-hidden="true" />
        </div>
      </div>

      {/* File info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span className="font-medium text-gray-900 dark:text-white truncate" title={file.name}>
            {escapeHtml(file.name)}
          </span>
          <Badge variant={statusConfig.variant} size="sm">
            {t(statusConfig.label)}
          </Badge>
        </div>

        {/* Preview / New name */}
        {newName && (isPreview || file.status === 'completed') && (
          <div className="flex items-center gap-2 text-sm mb-2">
            <ChevronRight className="w-4 h-4 text-gray-400 flex-shrink-0" aria-hidden="true" />
            <span className={clsx(
              'font-mono truncate',
              isPreview ? 'text-blue-600 dark:text-blue-400' : 'text-green-600 dark:text-green-400'
            )}>
              {escapeHtml(newName)}
            </span>
          </div>
        )}

        {/* Suggestions */}
        {suggestions.length > 0 && isPreview && (
          <div className="mt-2 flex flex-wrap gap-2">
            <span className="text-xs text-gray-500 dark:text-gray-400 flex items-center">
              {t('files.suggestions')}
            </span>
            {suggestions.map((suggestion, index) => (
              <Button
                key={suggestion}
                variant="ghost"
                size="sm"
                onClick={() => onSelectSuggestion(file.id, suggestion)}
                className="text-xs font-mono px-2 py-1 h-auto"
                title={suggestionLanguages[index] ? t('accessibility.suggestionButton', { name: suggestion, lang: suggestionLanguages[index] }) : undefined}
              >
                {escapeHtml(truncate(suggestion, 30))}
              </Button>
            ))}
          </div>
        )}

        {/* Error */}
        {error && file.status === 'failed' && (
          <div className="mt-2 flex items-center gap-2 text-sm text-red-600 dark:text-red-400">
            <AlertCircle className="w-4 h-4 flex-shrink-0" aria-hidden="true" />
            <span className="truncate">{escapeHtml(error)}</span>
          </div>
        )}

        {/* Warnings */}
        {warnings.length > 0 && file.status !== 'failed' && (
          <div className="mt-2 flex items-center gap-2 text-sm text-yellow-600 dark:text-yellow-400">
            <AlertTriangle className="w-4 h-4 flex-shrink-0" aria-hidden="true" />
            <span className="truncate">{warnings.map(escapeHtml).join('; ')}</span>
          </div>
        )}
      </div>
    </div>
  );
}