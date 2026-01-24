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
      set({ isOpen: true, message, resolve });
    });
  },

  close: (result: boolean) => {
    const { resolve } = get();
    if (resolve) resolve(result);
    set({ isOpen: false, message: "", resolve: null });
  },
}));
