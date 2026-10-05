/**
 * File list container
 */
import { useTranslation } from 'react-i18next';
import { useAppStore } from '@/store';
import { useRename } from '@/hooks/useRename';
import { useDragDrop } from '@/hooks/useDragDrop';
import { pickFiles, pickFolder } from '@/services/filepicker';
import { FileRow } from './FileRow';
import { DropZone } from './DropZone';
import { Button } from '@/components/ui';
import { Loader2, RotateCcw, Plus, X, Undo2 } from 'lucide-react';

export function FileList() {
  const { t } = useTranslation();
  const {
    files,
    processing,
    progress,
    lastResult,
    statusError,
    addFiles,
    clearFiles,
  } = useAppStore();

  const {
    runRename,
    handleCancel,
    handleUndo,
    canRun,
    canUndo,
    canCancel,
  } = useRename();

  const hasResults = !!lastResult;
  const pendingCount = files.filter(f => f.status === 'pending' || f.status === 'skipped').length;
  const dragActiveRef = { current: false };

  useDragDrop({
    onHover: (hovering) => {
      dragActiveRef.current = hovering;
      // Force re-render by updating a dummy state
      useAppStore.setState(state => state);
    },
  });

  const handleBrowseFiles = async () => {
    const picked = await pickFiles();
    if (picked.length > 0) addFiles(picked);
  };

  const handleBrowseFolder = async () => {
    const picked = await pickFolder();
    if (picked === null) return;
    if (picked.length > 0) {
      addFiles(picked);
    } else {
      // Toast handled by addFiles
    }
  };

  const handleSelectSuggestion = (fileId: string, suggestionName: string) => {
    const file = files.find(f => f.id === fileId);
    if (!file) return;

    const lastSep = Math.max(file.path.lastIndexOf('/'), file.path.lastIndexOf('\\'));
    const parentDir = lastSep >= 0 ? file.path.slice(0, lastSep + 1) : '';
    const newPath = parentDir + suggestionName;

    useAppStore.setState(state => ({
      files: state.files.map(f =>
        f.id === fileId
          ? { ...f, result: { ...f.result!, new_name: suggestionName, new_path: newPath } }
          : f
      ),
    }));
  };

  if (files.length === 0) {
    return (
      <div className="flex-1 flex flex-col">
        <DropZone
          onBrowseFiles={handleBrowseFiles}
          onBrowseFolder={handleBrowseFolder}
          dragActive={dragActiveRef.current}
        />
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col min-h-0">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-gray-900 dark:text-white">
            {files.length} {files.length === 1 ? t('files.fileCount', { count: files.length }) : t('files.fileCount_plural', { count: files.length })}
          </span>
          {processing && (
            <span className="text-sm text-blue-600 dark:text-blue-400 flex items-center gap-1">
              <Loader2 className="w-4 h-4 animate-spin" aria-hidden="true" />
              {progress || t('files.progress')}
            </span>
          )}
        </div>
        {statusError && (
          <span className="text-sm text-red-600 dark:text-red-400 flex items-center gap-1">
            <X className="w-4 h-4" aria-hidden="true" />
            {statusError}
          </span>
        )}
      </div>

      {/* File List */}
      <div className="flex-1 overflow-y-auto space-y-3" role="list" aria-label={t('accessibility.fileList')}>
        {files.map((file) => (
          <FileRow
            key={file.id}
            file={file}
            onSelectSuggestion={handleSelectSuggestion}
          />
        ))}
      </div>

      {/* Actions */}
      <div className="flex flex-wrap items-center justify-between gap-3 mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
        <div className="flex flex-wrap items-center gap-2">
          {hasResults ? (
            <>
              {canUndo && (
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={handleUndo}
                  aria-label={t('files.actions.undo')}
                >
                  <Undo2 className="w-4 h-4 mr-1.5" aria-hidden="true" />
                  {t('files.actions.undo')}
                </Button>
              )}
              <Button
                variant="secondary"
                size="sm"
                onClick={handleBrowseFiles}
                disabled={processing}
                aria-label={t('files.actions.addMore')}
              >
                <Plus className="w-4 h-4 mr-1.5" aria-hidden="true" />
                {t('files.actions.addMore')}
              </Button>
            </>
          ) : (
            <>
              <Button
                variant="danger"
                size="sm"
                onClick={handleCancel}
                disabled={!canCancel}
                aria-label={t('files.actions.cancel')}
              >
                <X className="w-4 h-4 mr-1.5" aria-hidden="true" />
                {t('files.actions.cancel')}
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => runRename(true)}
                disabled={!canRun || processing}
                aria-label={t('files.actions.dryRun')}
              >
                <RotateCcw className="w-4 h-4 mr-1.5" aria-hidden="true" />
                {t('files.actions.dryRun')}
              </Button>
              <Button
                variant="primary"
                size="sm"
                onClick={() => runRename(false)}
                disabled={!canRun || processing}
                aria-label={t('files.actions.rename', { count: pendingCount })}
              >
                {t('files.actions.rename', { count: pendingCount })}
              </Button>
            </>
          )}
        </div>

        <Button
          variant="ghost"
          size="sm"
          onClick={clearFiles}
          disabled={processing}
          aria-label={t('files.actions.clear')}
        >
          <X className="w-4 h-4 mr-1.5" aria-hidden="true" />
          {t('files.actions.clear')}
        </Button>
      </div>
    </div>
  );
}