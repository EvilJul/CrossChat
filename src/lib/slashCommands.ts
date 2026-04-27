/**
 * 斜杠命令系统 — 模块化注册机制
 *
 * 使用方式：
 *   1. 在输入框输入 / 触发命令菜单
 *   2. 选择命令或继续输入匹配命令名
 *   3. 按 Enter 执行命令
 *
 * 扩展方式：
 *   import { registerCommand } from "./slashCommands";
 *   registerCommand({ name: "mytool", description: "我的工具", handler: async () => { ... } });
 */

export interface SlashCommand {
  name: string;
  description: string;
  /** 是否需要在执行后清空输入 */
  clearInput?: boolean;
  handler: () => Promise<string | void> | string | void;
}

// 命令注册表
const registry = new Map<string, SlashCommand>();

// 订阅者（用于通知 UI 更新）
type Listener = () => void;
const listeners = new Set<Listener>();

export function subscribe(cb: Listener) {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

function notify() {
  listeners.forEach((cb) => cb());
}

/** 注册命令 */
export function registerCommand(cmd: SlashCommand) {
  registry.set(cmd.name, cmd);
  notify();
}

/** 注销命令 */
export function unregisterCommand(name: string) {
  registry.delete(name);
  notify();
}

/** 获取所有已注册命令 */
export function getCommands(): SlashCommand[] {
  return Array.from(registry.values()).sort((a, b) => a.name.localeCompare(b.name));
}

/** 匹配命令 */
export function matchCommands(prefix: string): SlashCommand[] {
  if (!prefix.startsWith("/")) return [];
  const query = prefix.slice(1).toLowerCase();
  return getCommands().filter((cmd) => cmd.name.toLowerCase().startsWith(query));
}

/** 执行命令。返回: undefined=命令不存在, ""=已执行无输出, "text"=已执行有输出 */
export async function executeCommand(input: string): Promise<string | undefined> {
  if (!input.startsWith("/")) return undefined;
  const parts = input.slice(1).split(/\s+/);
  const cmdName = parts[0].toLowerCase();
  const cmd = registry.get(cmdName);
  if (!cmd) return undefined;
  try {
    const result = await cmd.handler();
    return result ?? "";
  } catch (e) {
    return `命令执行失败: ${e}`;
  }
}
