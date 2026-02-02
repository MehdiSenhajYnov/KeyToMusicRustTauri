import { create } from "zustand";

interface ConfirmState {
  isOpen: boolean;
  message: string;
  resolve: ((value: boolean) => void) | null;
  confirm: (message: string) => Promise<boolean>;
  close: (result: boolean) => void;
}

export const useConfirmStore = create<ConfirmState>((set, get) => ({
  isOpen: false,
  message: "",
  resolve: null,

  confirm: (message: string) => {
    return new Promise<boolean>((resolve) => {
      const timeout = setTimeout(() => {
        get().close(false);
      }, 30000);

      const wrappedResolve = (value: boolean) => {
        clearTimeout(timeout);
        resolve(value);
      };

      set({ isOpen: true, message, resolve: wrappedResolve });
    });
  },

  close: (result: boolean) => {
    const { resolve } = get();
    if (resolve) resolve(result);
    set({ isOpen: false, message: "", resolve: null });
  },
}));
