import { getCurrentWebview } from '@tauri-apps/api/webview';
import { expandFolder } from './filepicker';
import { isSupportedFile } from './utils';

/**
 * Wire the Tauri drag-and-drop event to the app.
 *
 * `onDrop` receives supported files only; folders in the drop are expanded
 * recursively. Unsupported files and unreadable paths are silently skipped —
 * `onDrop` is called with an empty list when nothing usable was dropped, which
 * the caller reports to the user.
 */
export function setupDragDrop(
  onDrop: (paths: string[]) => void,
  onHover: (hovering: boolean) => void,
): () => void {
  let unlisten: (() => void) | undefined;
  let disposed = false;

  getCurrentWebview()
    .onDragDropEvent(async (event) => {
      switch (event.payload.type) {
        case 'over':
          onHover(true);
          break;
        case 'drop': {
          onHover(false);
          try {
            const dropped = event.payload.paths ?? [];
            const files: string[] = [];
            for (const path of dropped) {
              if (isSupportedFile(path)) {
                files.push(path);
                continue;
              }
              // Not a supported file: it may still be a folder to expand.
              // `expandFolder` resolves to `[]` for anything it cannot read.
              files.push(...(await expandFolder(path)));
            }
            // De-duplicate: the same file can arrive twice via a folder and a
            // direct drop.
            const unique = [...new Set(files)];
            if (!disposed) onDrop(unique);
          } catch {
            if (!disposed) onDrop([]);
          }
          break;
        }
        case 'leave':
          onHover(false);
          break;
      }
    })
    .then((fn) => {
      // The component may have been destroyed while the listener was being
      // registered; drop the listener immediately in that case.
      if (disposed) fn();
      else unlisten = fn;
    })
    .catch(() => {
      /* drag-and-drop is unavailable (e.g. running in a plain browser) */
    });

  return () => {
    disposed = true;
    unlisten?.();
  };
}
