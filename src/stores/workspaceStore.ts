import { create } from "zustand";

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

interface WorkspaceStore {
  currentDir: string;
  files: FileEntry[];
  isSidebarOpen: boolean;
  setCurrentDir: (dir: string) => void;
  setFiles: (files: FileEntry[]) => void;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
}

export const useWorkspaceStore = create<WorkspaceStore>((set) => ({
  currentDir: "",
  files: [],
  isSidebarOpen: false,
  setCurrentDir: (currentDir) => set({ currentDir, isSidebarOpen: true }),
  setFiles: (files) => set({ files }),
  toggleSidebar: () => set((s) => ({ isSidebarOpen: !s.isSidebarOpen })),
  setSidebarOpen: (isSidebarOpen) => set({ isSidebarOpen }),
}));
