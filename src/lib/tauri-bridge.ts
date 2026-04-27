import { Channel, invoke } from "@tauri-apps/api/core";
import type { StreamChunk } from "./providers";
import type { FileEntry } from "../stores/workspaceStore";

// 将前端消息格式转为 Rust 需要的 UnifiedMessage 格式
function toUnifiedMessages(messages: Array<{ role: string; content: string }>) {
  return messages.map((m) => ({
    role: m.role,
    content: [{ type: "text", text: m.content }],
  }));
}

export interface ProviderConfig {
  apiBase: string;
  apiKey: string;
  providerType: string;
}

// 列出目录内容
export async function listDirectory(path: string): Promise<FileEntry[]> {
  return (await invoke("list_directory", { path })) as FileEntry[];
}

// 获取用户主目录
export async function getHomeDir(): Promise<string> {
  return (await invoke("get_home_dir")) as string;
}

// === 会话管理 ===
export interface SessionMeta {
  id: string;
  title: string;
  created_at: number;
  updated_at: number;
  message_count: number;
}

export interface SessionMessage {
  role: string;
  content: string;
  timestamp: number;
}

export interface Session {
  meta: SessionMeta;
  messages: SessionMessage[];
  summary?: string | null;
}

export async function createSession(title: string): Promise<SessionMeta> {
  return (await invoke("create_session", { title })) as SessionMeta;
}

export async function listSessions(): Promise<SessionMeta[]> {
  return (await invoke("list_sessions")) as SessionMeta[];
}

export async function getSession(id: string): Promise<Session> {
  return (await invoke("get_session", { id })) as Session;
}

export async function saveMessages(
  sessionId: string,
  messages: SessionMessage[],
  summary?: string | null
): Promise<void> {
  await invoke("save_messages", { sessionId, messages, summary });
}

export async function deleteSession(id: string): Promise<void> {
  await invoke("delete_session", { id });
}

// === MCP 插件管理 ===
export interface McpServerConfig {
  id: string;
  name: string;
  command: string;
  args: string[];
  enabled: boolean;
}

export async function addMcpServer(config: McpServerConfig): Promise<void> {
  await invoke("add_mcp_server", { config });
}

export async function removeMcpServer(id: string): Promise<void> {
  await invoke("remove_mcp_server", { id });
}

export async function toggleMcpServer(id: string, enabled: boolean): Promise<void> {
  await invoke("toggle_mcp_server", { id, enabled });
}

export async function listMcpServers(): Promise<McpServerConfig[]> {
  return (await invoke("list_mcp_servers")) as McpServerConfig[];
}

export async function refreshMcpTools(): Promise<number> {
  return (await invoke("refresh_mcp_tools")) as number;
}

// === AGENT.md 约束文件 ===
export interface AgentConfig {
  found: boolean;
  global_content: string;
  global_path: string;
  workspace_content: string;
  workspace_path: string;
  merged: string;
}

export async function readAgentConfig(workDir?: string): Promise<AgentConfig> {
  return (await invoke("read_agent_config", { workDir })) as AgentConfig;
}

export async function writeGlobalAgentConfig(content: string): Promise<void> {
  await invoke("write_global_agent_config", { content });
}

// === 检查点（中断/继续） ===
export interface CheckpointMessage {
  role: string;
  content: string;
  thinking?: string | null;
}

export interface Checkpoint {
  messages: CheckpointMessage[];
  provider_id: string;
  model: string;
  work_dir: string;
  saved_at: number;
}

export async function saveCheckpoint(checkpoint: Checkpoint): Promise<void> {
  await invoke("save_checkpoint", { checkpoint });
}

export async function loadCheckpoint(): Promise<Checkpoint | null> {
  return (await invoke("load_checkpoint")) as Checkpoint | null;
}

export async function clearCheckpoint(): Promise<void> {
  await invoke("clear_checkpoint");
}

// === Skills ===
export interface SkillInfo {
  name: string;
  description: string;
  category: string;
  enabled: boolean;
}

export async function getAvailableSkills(): Promise<SkillInfo[]> {
  return (await invoke("get_available_skills")) as SkillInfo[];
}

// 精选 MCP 插件市场
export const MCP_MARKETPLACE: Array<{
  name: string;
  description: string;
  command: string;
  args: string[];
}> = [
  {
    name: "Filesystem",
    description: "安全地访问和修改文件系统",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-filesystem", "."],
  },
  {
    name: "GitHub",
    description: "管理 GitHub 仓库、Issues、PR",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-github"],
  },
  {
    name: "Postgres",
    description: "只读 PostgreSQL 数据库访问",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-postgres"],
  },
  {
    name: "Brave Search",
    description: "通过 Brave API 进行网页搜索",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-brave-search"],
  },
  {
    name: "Puppeteer",
    description: "浏览器自动化（截图、爬取）",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-puppeteer"],
  },
  {
    name: "Sequential Thinking",
    description: "增强的逐步推理能力",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-sequential-thinking"],
  },
  {
    name: "Memory",
    description: "基于知识图谱的持久记忆",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-memory"],
  },
  {
    name: "Fetch",
    description: "获取网页内容并转为 Markdown",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-fetch"],
  },
];

// 测试连接并拉取模型列表
export async function fetchModels(
  apiBase: string,
  apiKey: string,
  providerType: string
): Promise<string[]> {
  return (await invoke("test_provider_connection", {
    apiBase,
    apiKey,
    providerType,
  })) as string[];
}

// Tauri Channel 流式聊天调用
export async function streamChat(
  providerId: string,
  model: string,
  messages: Array<{ role: string; content: string }>,
  providerConfig: ProviderConfig | null,
  workDir: string,
  onChunk: (chunk: StreamChunk) => void,
  onError: (error: string) => void,
  onDone: (finishReason?: string) => void
): Promise<void> {
  const channel = new Channel<StreamChunk>();
  channel.onmessage = (chunk: StreamChunk) => {
    onChunk(chunk);
  };

  try {
    await invoke("stream_chat", {
      request: {
        provider_id: providerId,
        model,
        messages: toUnifiedMessages(messages),
        api_base: providerConfig?.apiBase ?? null,
        api_key: providerConfig?.apiKey ?? null,
        provider_type: providerConfig?.providerType ?? null,
        work_dir: workDir || null,
      },
      channel,
    });
    onDone("stop");
  } catch (e) {
    onError(String(e));
  }
}
