/**
 * Custom hook for rename operations
 */
import { useCallback } from 'react';
import { useAppStore } from '@/store';
import { renameFiles, cancelRename, undoRename } from '@/services/api';
import { showToast } from '@/hooks/useToast';
import { t } from '@/hooks/useTranslation';
import type { RenameOptions } from '@/types';

export function useRename() {
  const {
    files,
    processing,
    setProcessing,
    setResults,
    setStatusError,
    lastBatchId,
    clearFiles,
    config,
  } = useAppStore();

  const runRename = useCallback(async (dryRun: boolean) => {
    if (processing) return;

    const targets = files.filter(f => f.status === 'pending' || f.status === 'skipped');
    const paths = targets.map(f => f.path);

    if (paths.length === 0) {
      showToast(t('toasts.noFiles'), 'warning');
      return;
    }

    setProcessing(true, dryRun ? t('files.progress') : t('files.progress'));
    setStatusError('');

    // Mark target files as processing
    useAppStore.setState(state => ({
      files: state.files.map(f =>
        paths.includes(f.path) ? { ...f, status: 'processing' } : f
      ),
    }));

    try {
      const options: RenameOptions = {
        dryRun,
        provider: config?.ai.provider,
      };

      const result = await renameFiles(paths, options);

      // Check if cancelled during request
      if (useAppStore.getState().statusError === 'cancelled') return;

      setResults(result, dryRun);

      if (dryRun) {
        if (result.completed === 0 && result.skipped > 0) {
          showToast(t('toasts.renamePreviewAllSkipped'), 'info');
        } else {
          showToast(t('toasts.renamePreview', { completed: result.completed, skipped: result.skipped }), 'info');
        }
      } else {
        if (result.failed > 0) {
          showToast(t('toasts.renamePartial', { completed: result.completed, failed: result.failed }), 'warning');
        } else if (result.completed === 0 && result.skipped > 0) {
          showToast(t('toasts.renameAllSkipped'), 'info');
        } else {
          showToast(t('toasts.renameSuccess', { count: result.completed }), 'success');
        }

        // Show warnings (limited)
        const uniqueWarnings = [...new Set(result.files.flatMap(f => f.warnings ?? []))];
        const MAX_WARNINGS = 5;
        uniqueWarnings.slice(0, MAX_WARNINGS).forEach(w => showToast(w, 'warning'));
        if (uniqueWarnings.length > MAX_WARNINGS) {
          showToast(t('toasts.moreWarnings', { count: uniqueWarnings.length - MAX_WARNINGS }), 'warning');
        }
      }

      setProcessing(false);
    } catch (error) {
      if (useAppStore.getState().statusError === 'cancelled') return;

      const message = error instanceof Error ? error.message : String(error);
      setStatusError(message);
      setProcessing(false);

      // Mark processing files as failed
      useAppStore.setState(state => ({
        files: state.files.map(f =>
          f.status === 'processing' ? { ...f, status: 'failed' } : f
        ),
      }));

      showToast(t('toasts.renameFailed', { error: message }), 'danger');
    }
  }, [files, processing, config?.ai.provider, setProcessing, setResults, setStatusError]);

  const handleCancel = useCallback(async () => {
    if (!processing) return;

    await cancelRename();
    setStatusError('cancelled');

    useAppStore.setState(state => ({
      processing: false,
      progress: '',
      files: state.files.map(f =>
        f.status === 'processing' ? { ...f, status: 'failed' } : f
      ),
    }));

    showToast(t('toasts.cancelled'), 'info');
  }, [processing]);

  const handleUndo = useCallback(async () => {
    if (!lastBatchId || processing) return;

    setProcessing(true, t('files.progress'));

    try {
      const result = await undoRename(lastBatchId);
      setProcessing(false);

      if (result.success) {
        showToast(t('toasts.undoSuccess', { count: result.restored }), 'success');
        clearFiles();
      } else if (result.failed > 0) {
        showToast(t('toasts.undoPartial', { restored: result.restored, failed: result.failed }), 'warning');
      } else {
        showToast(t('toasts.undoNothing'), 'info');
      }
    } catch (error) {
      setProcessing(false);
      const message = error instanceof Error ? error.message : String(error);
      showToast(t('toasts.undoFailed', { error: message }), 'danger');
    }
  }, [lastBatchId, processing, setProcessing, clearFiles]);

  return {
    runRename,
    handleCancel,
    handleUndo,
    canRun: files.some(f => f.status === 'pending' || f.status === 'skipped') && !processing,
    canUndo: !!lastBatchId && !processing,
    canCancel: processing,
  };
}