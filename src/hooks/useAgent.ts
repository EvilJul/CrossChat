import { useCallback } from "react";
import { readAgentConfig, writeGlobalAgentConfig } from "../lib/tauri-bridge";

export function useAgent() {
  const load = useCallback(async (workDir?: string) => {
    try {
      return await readAgentConfig(workDir);
    } catch {
      return null;
    }
  }, []);

  const saveGlobal = useCallback(async (content: string) => {
    await writeGlobalAgentConfig(content);
  }, []);

  return { load, saveGlobal };
}
