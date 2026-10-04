/**
 * Views - History View
 */
import { useTranslation } from 'react-i18next';
import { Clock, FileText, CheckCircle, XCircle, AlertCircle } from 'lucide-react';
import { Button, Card, Badge } from '@/components/ui';
import { useAppStore } from '@/store';

const STAT_CARD_COLORS = {
  blue: 'bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400',
  green: 'bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400',
  yellow: 'bg-yellow-50 dark:bg-yellow-900/20 text-yellow-600 dark:text-yellow-400',
  red: 'bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400',
} as const;

interface StatCardProps {
  label: string;
  value: number;
  icon: React.ReactNode;
  color?: keyof typeof STAT_CARD_COLORS;
}

function StatCard({ label, value, icon, color = 'blue' }: StatCardProps) {
  return (
    <div className={`p-4 rounded-xl text-center ${STAT_CARD_COLORS[color]}`}>
      <div className="flex items-center justify-center mb-2">
        {icon}
      </div>
      <p className="text-2xl font-bold">{value}</p>
      <p className="text-sm">{label}</p>
    </div>
  );
}

interface FileHistoryRowProps {
  file: {
    file: string;
    new_name?: string | null;
    status: string;
  };
  index: number;
}

function FileHistoryRow({ file, index }: FileHistoryRowProps) {
  return (
    <div className="flex items-center gap-3 p-3 rounded-lg bg-gray-50 dark:bg-gray-800/50">
      <span className="text-sm text-gray-500 dark:text-gray-400 w-6 text-right">{index + 1}.</span>
      <span className="flex-1 truncate font-mono text-sm">{file.file.split(/[\\/]/).pop()}</span>
      {file.new_name && (
        <span className="flex-1 truncate font-mono text-sm text-blue-600 dark:text-blue-400">
          → {file.new_name}
        </span>
      )}
      <Badge variant={file.status as 'default' | 'success' | 'warning' | 'danger' | 'info' | 'processing'} size="sm">
        {file.status}
      </Badge>
    </div>
  );
}

export function HistoryView() {
  const { t } = useTranslation();
  const { lastResult, lastBatchId, clearFiles } = useAppStore();

  if (!lastResult || !lastBatchId) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-center">
        <Clock className="w-16 h-16 text-gray-300 dark:text-gray-600 mb-4" aria-hidden="true" />
        <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">
          {t('history.noHistory')}
        </h3>
        <p className="text-gray-500 dark:text-gray-400 mb-6">
          {t('history.noHistoryDesc') || 'No rename operations have been performed yet.'}
        </p>
        <Button variant="primary" onClick={() => useAppStore.getState().setView('files')}>
          {t('navigation.files')}
        </Button>
      </div>
    );
  }

  const batch = lastResult;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            {t('history.title')}
          </h2>
          <p className="text-gray-500 dark:text-gray-400 mt-1">
            {t('history.batch')} {batch.batch_id}
          </p>
        </div>
        <Button variant="secondary" onClick={clearFiles}>
          {t('common.clear')}
        </Button>
      </div>

      <Card>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
          <StatCard
            label={t('history.filesProcessed')}
            value={batch.total}
            icon={<FileText className="w-5 h-5" />}
          />
          <StatCard
            label={t('common.completed')}
            value={batch.completed}
            icon={<CheckCircle className="w-5 h-5" />}
            color="green"
          />
          <StatCard
            label={t('common.skipped')}
            value={batch.skipped}
            icon={<XCircle className="w-5 h-5" />}
            color="yellow"
          />
          <StatCard
            label={t('common.failed')}
            value={batch.failed}
            icon={<AlertCircle className="w-5 h-5" />}
            color="red"
          />
        </div>
      </Card>

      <Card>
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          {t('history.filesProcessed')}
        </h3>
        <div className="space-y-2 max-h-96 overflow-y-auto">
          {batch.files.map((file, index) => (
            <FileHistoryRow key={file.file} file={file} index={index} />
          ))}
        </div>
      </Card>

      {batch.dry_run && (
        <div className="p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-xl">
          <p className="text-blue-800 dark:text-blue-300">
            {t('toasts.renamePreview', { completed: batch.completed, skipped: batch.skipped })}
          </p>
        </div>
      )}
    </div>
  );
}
