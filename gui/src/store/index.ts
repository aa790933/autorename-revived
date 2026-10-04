/**
 * Zustand store for AutoRename-Revived v4.0.0
 * Centralized state management with React integration
 */
import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
import type {
  AppView,
  FileEntry,
  BatchResult,
  AppConfig,
  FileResult,
} from '@/types';
import { generateId } from '@/utils/helpers';

interface AppState {
  // View state
  view: AppView;
  setView: (view: AppView) => void;

  // File management
  files: FileEntry[];
  addFiles: (paths: string[]) => void;
  clearFiles: () => void;
  removeFile: (id: string) => void;
  updateFileStatus: (id: string, status: FileEntry['status'], result?: FileResult) => void;
  replaceFileResult: (id: string, result: FileResult) => void;

  // Processing state
  processing: boolean;
  progress: string;
  setProcessing: (processing: boolean, progress?: string) => void;

  // Results
  lastResult: BatchResult | null;
  dryRunResult: BatchResult | null;
  lastBatchId: string | null;
  setResults: (result: BatchResult, isDryRun: boolean) => void;

  // Error state
  statusError: string;
  setStatusError: (error: string) => void;

  // Config
  config: AppConfig | null;
  setConfig: (config: AppConfig) => void;
  updateConfig: (partial: Partial<AppConfig>) => void;

  // UI state
  sidebarOpen: boolean;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;

  // Theme
  theme: 'light' | 'dark' | 'system';
  setTheme: (theme: 'light' | 'dark' | 'system') => void;

  // Language
  language: string;
  setLanguage: (language: string) => void;

  // Reset all state
  reset: () => void;
}

const initialState = {
  view: 'files' as AppView,
  files: [],
  processing: false,
  progress: '',
  lastResult: null,
  dryRunResult: null,
  lastBatchId: null,
  statusError: '',
  config: null,
  sidebarOpen: true,
  theme: 'system' as const,
  language: 'en',
};

export const useAppStore = create<AppState>()(
  subscribeWithSelector((set, get) => ({
    ...initialState,

    // View actions
    setView: (view) => set({ view }),

    // File actions
    addFiles: (paths) => {
      const existingPaths = new Set(get().files.map(f => f.path));
      const newFiles: FileEntry[] = paths
        .filter(p => !existingPaths.has(p))
        .map(p => ({
          id: generateId(),
          path: p,
          name: p.split(/[\\/]/).pop() || p,
          status: 'pending' as const,
        }));
      set(state => ({
        files: [...state.files, ...newFiles],
        dryRunResult: null,
        lastResult: null,
        statusError: '',
        progress: '',
      }));
    },

    clearFiles: () => set({
      files: [],
      dryRunResult: null,
      lastResult: null,
      progress: '',
      statusError: '',
      lastBatchId: null,
    }),

    removeFile: (id) => set(state => ({
      files: state.files.filter(f => f.id !== id),
    })),

    updateFileStatus: (id, status, result) => set(state => ({
      files: state.files.map(f =>
        f.id === id ? { ...f, status, result } : f
      ),
    })),

    replaceFileResult: (id, result) => set(state => ({
      files: state.files.map(f =>
        f.id === id ? { ...f, result } : f
      ),
    })),

    // Processing actions
    setProcessing: (processing, progress = '') => set({ processing, progress }),

    // Results actions
    setResults: (result, isDryRun) => set(_state => {
      if (isDryRun) {
        return { dryRunResult: result };
      }
      return {
        lastResult: result,
        lastBatchId: result.completed > 0 ? result.batch_id : null,
      };
    }),

    // Error actions
    setStatusError: (error) => set({ statusError: error }),

    // Config actions
    setConfig: (config) => set({ config }),
    updateConfig: (partial) => set(state => ({
      config: state.config ? { ...state.config, ...partial } : null,
    })),

    // UI actions
    toggleSidebar: () => set(state => ({ sidebarOpen: !state.sidebarOpen })),
    setSidebarOpen: (open) => set({ sidebarOpen: open }),

    // Theme actions
    setTheme: (theme) => set({ theme }),

    // Language actions
    setLanguage: (language) => set({ language }),

    // Reset
    reset: () => set(initialState),
  }))
);

// Selectors for common derived state
export const selectPendingFiles = (state: AppState) =>
  state.files.filter(f => f.status === 'pending' || f.status === 'skipped');

export const selectProcessingFiles = (state: AppState) =>
  state.files.filter(f => f.status === 'processing');

export const selectCompletedFiles = (state: AppState) =>
  state.files.filter(f => f.status === 'completed');

export const selectFailedFiles = (state: AppState) =>
  state.files.filter(f => f.status === 'failed');

export const selectFileCount = (state: AppState) => state.files.length;

export const selectHasResults = (state: AppState) => state.lastResult !== null;