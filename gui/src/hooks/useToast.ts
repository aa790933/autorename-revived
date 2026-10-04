/**
 * Toast notification system
 */
import { create } from 'zustand';
import { generateId } from '@/utils/helpers';

export type ToastType = 'success' | 'danger' | 'warning' | 'info';

export interface Toast {
  id: string;
  message: string;
  type: ToastType;
  duration?: number;
}

interface ToastState {
  toasts: Toast[];
  addToast: (message: string, type: ToastType, duration?: number) => void;
  removeToast: (id: string) => void;
  clearToasts: () => void;
}

export const useToastStore = create<ToastState>((set) => ({
  toasts: [],

  addToast: (message, type, duration = 5000) => {
    const id = generateId();
    set(state => ({ toasts: [...state.toasts, { id, message, type, duration }] }));

    if (duration > 0) {
      setTimeout(() => {
        set(state => ({ toasts: state.toasts.filter(t => t.id !== id) }));
      }, duration);
    }
  },

  removeToast: (id) => set(state => ({
    toasts: state.toasts.filter(t => t.id !== id),
  })),

  clearToasts: () => set({ toasts: [] }),
}));

/**
 * Hook for showing toasts (convenience wrapper)
 */
export function showToast(message: string, type: ToastType = 'info', duration = 5000): void {
  useToastStore.getState().addToast(message, type, duration);
}