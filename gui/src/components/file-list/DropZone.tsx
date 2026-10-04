/**
 * File List Components - Drop Zone
 */
import { useTranslation } from 'react-i18next';
import { FileText, Upload, FolderOpen, Plus } from 'lucide-react';
import { Button } from '@/components/ui';
import { useDragDrop } from '@/hooks/useDragDrop';
import { pickFiles, pickFolder } from '@/services/filepicker';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

interface DropZoneProps {
  onBrowseFiles: () => void;
  onBrowseFolder: () => void;
  dragActive?: boolean;
}

export function DropZone({ onBrowseFiles, onBrowseFolder, dragActive }: DropZoneProps) {
  const { t } = useTranslation();
  const { cleanup } = useDragDrop({ onHover: () => {} }); // Handled by parent

  return (
    <div
      className={twMerge(
        'flex flex-col items-center justify-center gap-6 p-12 w-full max-w-2xl mx-auto',
        'border-2 border-dashed rounded-2xl transition-all duration-300',
        dragActive
          ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
          : 'border-gray-300 dark:border-gray-600 hover:border-blue-400 dark:hover:border-blue-500'
      )}
      role="button"
      tabIndex={0}
      onClick={(e) => {
        if (!(e.target as HTMLElement).closest('button')) {
          onBrowseFiles();
        }
      }}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onBrowseFiles();
        }
      }}
      aria-label={t('files.dropZone.title')}
    >
      <div className="flex flex-col items-center gap-4">
        <FileText className={clsx(
          'w-16 h-16 transition-colors',
          dragActive ? 'text-blue-500' : 'text-gray-400 dark:text-gray-500'
        )} aria-hidden="true" />
        
        <div className="text-center">
          <p className="text-lg font-medium text-gray-900 dark:text-white">
            {t('files.dropZone.title')}
          </p>
          <p className="text-gray-500 dark:text-gray-400 mt-1">
            {t('files.dropZone.subtitle')}
          </p>
          <p className="text-sm text-gray-400 dark:text-gray-500 mt-1">
            {t('files.dropZone.formats')}
          </p>
        </div>

        <div className="flex gap-3 mt-2">
          <Button 
            variant="primary" 
            size="md"
            onClick={(e) => { e.stopPropagation(); onBrowseFiles(); }}
            aria-label={t('files.dropZone.browseFiles')}
          >
            <Upload className="w-4 h-4 mr-2" aria-hidden="true" />
            {t('files.dropZone.browseFiles')}
          </Button>
          <Button 
            variant="secondary" 
            size="md"
            onClick={(e) => { e.stopPropagation(); onBrowseFolder(); }}
            aria-label={t('files.dropZone.browseFolder')}
          >
            <FolderOpen className="w-4 h-4 mr-2" aria-hidden="true" />
            {t('files.dropZone.browseFolder')}
          </Button>
        </div>
      </div>
    </div>
  );
}