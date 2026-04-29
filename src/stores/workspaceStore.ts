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
  selectedFile: string | null;
  previewOpen: boolean;
  refreshTrigger: number;
  setCurrentDir: (dir: string) => void;
  setFiles: (files: FileEntry[]) => void;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
  setSelectedFile: (file: string | null) => void;
  setPreviewOpen: (open: boolean) => void;
  triggerRefresh: () => void;
}

export const useWorkspaceStore = create<WorkspaceStore>((set) => ({
  currentDir: "",
  files: [],
  isSidebarOpen: false,
  selectedFile: null,
  previewOpen: false,
  refreshTrigger: 0,
  setCurrentDir: (currentDir) => set({ currentDir, isSidebarOpen: true }),
  setFiles: (files) => set({ files }),
  toggleSidebar: () => set((s) => ({ isSidebarOpen: !s.isSidebarOpen })),
  setSidebarOpen: (isSidebarOpen) => set({ isSidebarOpen }),
  setSelectedFile: (selectedFile) => set({ selectedFile, previewOpen: selectedFile !== null }),
  setPreviewOpen: (previewOpen) => set({ previewOpen }),
  triggerRefresh: () => set((s) => ({ refreshTrigger: s.refreshTrigger + 1 })),
}));
