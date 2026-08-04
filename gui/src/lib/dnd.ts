import { getCurrentWebview } from '@tauri-apps/api/webview';
import { expandFolder } from './filepicker';
import { SUPPORTED_EXTENSIONS } from './utils';

function isSupportedFile(path: string): boolean {
  const ext = path.toLowerCase().replace(/.*[.](\w+)$/, '.$1');
  return SUPPORTED_EXTENSIONS.includes(ext);
}

export function setupDragDrop(
  onDrop: (paths: string[]) => void,
  onHover: (hovering: boolean) => void,
): () => void {
  let unlisten: (() => void) | undefined;

  getCurrentWebview().onDragDropEvent(async (event) => {
    switch (event.payload.type) {
      case 'over':
        onHover(true);
        break;
      case 'drop': {
        onHover(false);
        try {
          const dropped = event.payload.paths;
          const supportedFiles = dropped.filter((p: string) => isSupportedFile(p));
          const dirs = dropped.filter((p: string) => !isSupportedFile(p));

          const expanded = await Promise.all(
            dirs.map(async (p: string) => {
              try { return await expandFolder(p); }
              catch { return [] as string[]; }
            }),
          );

          const allFiles = [...supportedFiles, ...expanded.flat()].filter((p) => isSupportedFile(p));
          onDrop(allFiles);
        } catch {
          onDrop([]);
        }
        break;
      }
      case 'leave':
        onHover(false);
        break;
    }
  }).then((fn) => { unlisten = fn; });

  return () => unlisten?.();
}
