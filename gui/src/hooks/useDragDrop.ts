/**
 * Drag and drop hook for file handling
 */
import { useCallback, useEffect, useRef } from 'react';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { expandFolder } from '@/services/filepicker';
import { isSupportedFile } from '@/utils/helpers';
import { showToast } from '@/hooks/useToast';
import { t } from '@/hooks/useTranslation';
import { useAppStore } from '@/store';

interface UseDragDropOptions {
  onHover?: (hovering: boolean) => void;
}

export function useDragDrop({ onHover }: UseDragDropOptions = {}) {
  const addFiles = useAppStore(state => state.addFiles);
  const disposedRef = useRef(false);
  const unlistenRef = useRef<(() => void) | null>(null);

  const handleDrop = useCallback(async (paths: string[]) => {
    if (disposedRef.current) return;

    const files: string[] = [];
    for (const path of paths) {
      if (isSupportedFile(path)) {
        files.push(path);
        continue;
      }
      // Try to expand as folder
      try {
        const expanded = await expandFolder(path);
        files.push(...expanded);
      } catch {
        // Ignore unreadable paths
      }
    }

    // Deduplicate
    const unique = [...new Set(files)];

    if (unique.length === 0) {
      showToast(t('toasts.unsupportedFiles'), 'warning');
    } else {
      addFiles(unique);
      showToast(t('toasts.filesAdded', { count: unique.length }), 'success');
    }
  }, [addFiles]);

  useEffect(() => {
    let mounted = true;

    getCurrentWebview()
      .onDragDropEvent(async (event) => {
        if (!mounted || disposedRef.current) return;

        switch (event.payload.type) {
          case 'over':
            onHover?.(true);
            break;
          case 'drop': {
            onHover?.(false);
            const dropped = event.payload.paths ?? [];
            await handleDrop(dropped);
            break;
          }
          case 'leave':
            onHover?.(false);
            break;
        }
      })
      .then((fn) => {
        if (!mounted || disposedRef.current) {
          fn();
        } else {
          unlistenRef.current = fn;
        }
      })
      .catch(() => {
        // Drag and drop unavailable (e.g., in browser)
      });

    return () => {
      mounted = false;
      disposedRef.current = true;
      unlistenRef.current?.();
    };
  }, [handleDrop, onHover]);

  const cleanup = useCallback(() => {
    disposedRef.current = true;
    unlistenRef.current?.();
  }, []);

  return { cleanup };
}