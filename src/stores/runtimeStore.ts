import { create } from "zustand";
import type { InputRuntime } from "../types";

const defaultInputRuntime: InputRuntime = {
  isLinux: false,
  isWayland: false,
  browserKeyFallback: false,
};

interface RuntimeState {
  inputRuntime: InputRuntime;
  setInputRuntime: (inputRuntime: InputRuntime) => void;
}

export const useRuntimeStore = create<RuntimeState>((set) => ({
  inputRuntime: defaultInputRuntime,
  setInputRuntime: (inputRuntime) => set({ inputRuntime }),
}));
