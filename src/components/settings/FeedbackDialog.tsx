import { useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { X, Mail, Send, Loader2 } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useSettingsStore } from "../../stores/settingsStore";

// 邮箱地址不在 UI 中明文显示
const FEEDBACK_EMAIL = atob("MTQxOTY0ODcwMUBxcS5jb20=");

export default function FeedbackDialog() {
  const feedbackOpen = useSettingsStore((s) => s.feedbackOpen);
  const setFeedbackOpen = useSettingsStore((s) => s.setFeedbackOpen);
  const [feedback, setFeedback] = useState("");
  const [sending, setSending] = useState(false);
  const [sent, setSent] = useState(false);

  const handleSend = async () => {
    if (!feedback.trim()) return;
    setSending(true);

    const subject = encodeURIComponent("[CrossChat 反馈] - " + new Date().toLocaleString("zh-CN"));
    const body = encodeURIComponent(
      feedback + "\n\n---\n系统信息:\n" +
      `平台: ${navigator.platform}\n` +
      `时间: ${new Date().toISOString()}\n` +
      `语言: ${navigator.language}\n`
    );
    const mailto = `mailto:${FEEDBACK_EMAIL}?subject=${subject}&body=${body}`;

    try {
      await openUrl(mailto);
      setSent(true);
      setTimeout(() => {
        setSent(false);
        setFeedback("");
        setFeedbackOpen(false);
      }, 2000);
    } catch {
      // 如果 openUrl 失败，复制内容到剪贴板
      await navigator.clipboard.writeText(feedback);
      setSent(true);
    }
    setSending(false);
  };

  return (
    <Dialog.Root open={feedbackOpen} onOpenChange={setFeedbackOpen}>
      <Dialog.Trigger asChild>
        <button
          className="text-xs text-ds-muted hover:text-ds-accent bg-ds-bg-main hover:bg-ds-hover px-2.5 py-1.5 rounded-lg transition-colors flex items-center gap-1"
          title="反馈建议 (测试阶段)"
        >
          <Mail className="w-3 h-3" />
          反馈
        </button>
      </Dialog.Trigger>

      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/40 backdrop-blur-sm z-50" />
        <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-ds-surface-card backdrop-blur-xl rounded-2xl shadow-2xl w-[420px] z-50 border border-ds-border text-ds-text-primary">
          <div className="flex items-center justify-between px-5 py-4 border-b border-ds-border">
            <Dialog.Title className="text-sm font-semibold text-ds-text-primary flex items-center gap-2">
              <Mail className="w-4 h-4 text-ds-accent" />
              用户反馈
            </Dialog.Title>
            <Dialog.Close className="p-1 rounded-lg text-ds-muted hover:text-ds-text-primary">
              <X className="w-4 h-4" />
            </Dialog.Close>
          </div>

          <div className="p-5 space-y-3">
            <p className="text-xs text-ds-muted">
              你的反馈将直接发送到开发者邮箱。包括建议、bug 报告、功能需求等。（测试阶段专属功能）
            </p>

            <textarea
              value={feedback}
              onChange={(e) => setFeedback(e.target.value)}
              placeholder="请描述你的问题、建议或反馈..."
              rows={5}
              className="w-full resize-none rounded-xl border border-ds-border bg-ds-surface-elevated px-4 py-3 text-sm text-ds-text-primary placeholder:text-ds-muted focus:outline-none focus:ring-2 focus:ring-ds-accent/30 focus:border-ds-accent"
            />

            <button
              onClick={handleSend}
              disabled={!feedback.trim() || sending}
              className="w-full py-2 rounded-xl bg-ds-accent hover:opacity-95 disabled:opacity-40 disabled:cursor-not-allowed text-white text-sm flex items-center justify-center gap-2 transition-all duration-200 shadow-md shadow-ds-accent/20"
            >
              {sent ? (
                <>已发送！</>
              ) : sending ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <>
                  <Send className="w-4 h-4" />
                  发送反馈
                </>
              )}
            </button>

            {sent && (
              <p className="text-xs text-ds-success text-center">
                邮件客户端已打开，请在邮件中点击发送完成反馈
              </p>
            )}
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
