import { create } from "zustand";
import type { ProviderEntry } from "./settingsStore";

interface ProviderStore {
  providers: ProviderEntry[];
  addProvider: (p: ProviderEntry) => void;
  removeProvider: (id: string) => void;
  updateProvider: (id: string, update: Partial<ProviderEntry>) => void;
  getProvider: (id: string) => ProviderEntry | undefined;
}

export const useProviderStore = create<ProviderStore>((set, get) => ({
  providers: [],

  addProvider: (p) =>
    set((s) => ({
      providers: [...s.providers, p],
    })),

  removeProvider: (id) =>
    set((s) => ({
      providers: s.providers.filter((p) => p.id !== id),
    })),

  updateProvider: (id, update) =>
    set((s) => ({
      providers: s.providers.map((p) =>
        p.id === id ? { ...p, ...update } : p
      ),
    })),

  getProvider: (id) => get().providers.find((p) => p.id === id),
}));
