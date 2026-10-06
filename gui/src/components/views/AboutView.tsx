/**
 * About view
 */
import { useTranslation } from 'react-i18next';
import { Github, AlertCircle, FileText, CheckCircle, Heart } from 'lucide-react';
import { Card, Badge } from '@/components/ui';
import { __APP_VERSION__ } from '@/utils/helpers';

export const FEATURES = [
  { icon: CheckCircle, labelKey: 'about.features.nativeBackend' },
  { icon: CheckCircle, labelKey: 'about.features.multiProvider' },
  { icon: CheckCircle, labelKey: 'about.features.localExtraction' },
  { icon: CheckCircle, labelKey: 'about.features.visionMode' },
  { icon: CheckCircle, labelKey: 'about.features.multiLanguage' },
  { icon: CheckCircle, labelKey: 'about.features.customTemplates' },
  { icon: CheckCircle, labelKey: 'about.features.undoHistory' },
  { icon: CheckCircle, labelKey: 'about.features.portableMode' },
];

const LINKS = [
  { icon: Github, label: 'about.links.github', href: 'https://github.com/aa790933/autorename-revived' },
  { icon: AlertCircle, label: 'about.links.issues', href: 'https://github.com/aa790933/autorename-revived/issues' },
  { icon: FileText, label: 'about.links.license', href: 'https://github.com/aa790933/autorename-revived/blob/main/LICENSE' },
];

export function AboutView() {
  const { t } = useTranslation();

  return (
    <div className="max-w-3xl mx-auto space-y-8">
      {/* Header */}
      <div className="text-center">
        <div className="inline-flex items-center justify-center w-20 h-20 rounded-2xl bg-blue-600 mb-6">
          <FileText className="w-10 h-10 text-white" aria-hidden="true" />
        </div>
        <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-2">
          {t('app.name')}
        </h1>
        <p className="text-gray-500 dark:text-gray-400 text-lg">
          {t('app.tagline')}
        </p>
        <Badge variant="info" className="mt-4" size="md">
          {t('app.version', { version: __APP_VERSION__ })}
        </Badge>
      </div>

      {/* Description */}
      <Card>
        <p className="text-gray-600 dark:text-gray-300 leading-relaxed">
          {t('about.description')}
        </p>
      </Card>

      {/* Features */}
      <Card>
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          {t('about.features')}
        </h3>
        <div className="grid gap-3 sm:grid-cols-2">
          {FEATURES.map(({ icon: Icon, labelKey }, index) => (
            <div key={index} className="flex items-center gap-3 p-3 rounded-lg bg-gray-50 dark:bg-gray-800/50">
              <Icon className="w-5 h-5 text-green-600 dark:text-green-400 flex-shrink-0" aria-hidden="true" />
              <span className="text-gray-700 dark:text-gray-300">{t(labelKey)}</span>
            </div>
          ))}
        </div>
      </Card>

      {/* Tech Stack */}
      <Card>
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Technology Stack
        </h3>
        <div className="flex flex-wrap gap-2">
          {[
            'Tauri v2',
            'Rust',
            'React 18',
            'TypeScript',
            'Tailwind CSS',
            'Zustand',
            'i18next',
            'Lucide Icons',
          ].map((tech, index) => (
            <Badge key={index} variant="default" size="sm">
              {tech}
            </Badge>
          ))}
        </div>
      </Card>

      {/* Links */}
      <Card>
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Links
        </h3>
        <div className="space-y-2">
          {LINKS.map(({ icon: Icon, label, href }, index) => (
            <a
              key={index}
              href={href}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-3 p-3 rounded-lg bg-gray-50 dark:bg-gray-800/50 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
            >
              <Icon className="w-5 h-5 text-gray-500 dark:text-gray-400 flex-shrink-0" aria-hidden="true" />
              <span className="text-gray-700 dark:text-gray-300">{t(label)}</span>
            </a>
          ))}
        </div>
      </Card>

      {/* Credits */}
      <div className="text-center text-gray-500 dark:text-gray-400 text-sm">
        <p>{t('about.credits')}</p>
        <p className="mt-2 flex items-center justify-center gap-1">
          Made with <Heart className="w-4 h-4 text-red-500" aria-hidden="true" /> by the AutoRename-Revived Contributors
        </p>
      </div>
    </div>
  );
}