import { useState, useEffect } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { X, ChevronRight, ChevronLeft, Check, MessageSquare, Settings, Zap, Shield } from "lucide-react";

interface WelcomeStep {
  title: string;
  description: string;
  icon: React.ReactNode;
  content: React.ReactNode;
}

const WELCOME_STEPS: WelcomeStep[] = [
  {
    title: "欢迎使用 CrossChat",
    description: "一个强大的跨平台聊天应用",
    icon: <MessageSquare className="w-8 h-8 text-purple-500" />,
    content: (
      <div className="space-y-4">
        <p className="text-sm text-zinc-600 dark:text-zinc-400">
          CrossChat 是一个功能丰富的聊天应用，支持多种AI模型、文件预览、Python脚本执行等功能。
        </p>
        <div className="bg-gradient-to-r from-purple-50 to-blue-50 dark:from-purple-900/20 dark:to-blue-900/20 p-3 rounded-lg border border-purple-200/50 dark:border-purple-800/50">
          <p className="text-xs bg-gradient-to-r from-purple-700 to-blue-700 dark:from-purple-300 dark:to-blue-300 bg-clip-text text-transparent font-medium">
            本指引将帮助您快速了解和使用 CrossChat 的主要功能。
          </p>
        </div>
      </div>
    ),
  },
  {
    title: "配置AI模型",
    description: "设置您的AI服务提供商",
    icon: <Settings className="w-8 h-8 text-blue-500" />,
    content: (
      <div className="space-y-4">
        <p className="text-sm text-zinc-600 dark:text-zinc-400">
          首先，您需要配置AI服务提供商。CrossChat 支持多种AI模型，包括 OpenAI、Claude、本地模型等。
        </p>
        <div className="space-y-2">
          <h4 className="text-xs font-medium text-zinc-700 dark:text-zinc-300">配置步骤：</h4>
          <ol className="text-xs text-zinc-600 dark:text-zinc-400 space-y-1 list-decimal list-inside">
            <li>点击右上角的设置按钮</li>
            <li>选择"模型"选项卡</li>
            <li>添加您的API密钥</li>
            <li>选择要使用的模型</li>
          </ol>
        </div>
      </div>
    ),
  },
  {
    title: "开始聊天",
    description: "创建您的第一个对话",
    icon: <Zap className="w-8 h-8 text-amber-500" />,
    content: (
      <div className="space-y-4">
        <p className="text-sm text-zinc-600 dark:text-zinc-400">
          配置完成后，您就可以开始与AI对话了。CrossChat 支持多种功能：
        </p>
        <div className="grid grid-cols-2 gap-2">
          <div className="bg-gradient-to-br from-purple-50 to-purple-100/50 dark:from-purple-900/20 dark:to-purple-800/10 p-2 rounded-lg border border-purple-200/50 dark:border-purple-800/30">
            <p className="text-xs font-medium text-purple-700 dark:text-purple-300">文件预览</p>
            <p className="text-xs text-zinc-500 dark:text-zinc-400">预览各种文件格式</p>
          </div>
          <div className="bg-gradient-to-br from-blue-50 to-blue-100/50 dark:from-blue-900/20 dark:to-blue-800/10 p-2 rounded-lg border border-blue-200/50 dark:border-blue-800/30">
            <p className="text-xs font-medium text-blue-700 dark:text-blue-300">Python脚本</p>
            <p className="text-xs text-zinc-500 dark:text-zinc-400">执行Python代码</p>
          </div>
          <div className="bg-gradient-to-br from-indigo-50 to-indigo-100/50 dark:from-indigo-900/20 dark:to-indigo-800/10 p-2 rounded-lg border border-indigo-200/50 dark:border-indigo-800/30">
            <p className="text-xs font-medium text-indigo-700 dark:text-indigo-300">MCP工具</p>
            <p className="text-xs text-zinc-500 dark:text-zinc-400">扩展AI功能</p>
          </div>
          <div className="bg-gradient-to-br from-violet-50 to-violet-100/50 dark:from-violet-900/20 dark:to-violet-800/10 p-2 rounded-lg border border-violet-200/50 dark:border-violet-800/30">
            <p className="text-xs font-medium text-violet-700 dark:text-violet-300">会话管理</p>
            <p className="text-xs text-zinc-500 dark:text-zinc-400">保存和恢复对话</p>
          </div>
        </div>
      </div>
    ),
  },
  {
    title: "安全提示",
    description: "了解安全使用注意事项",
    icon: <Shield className="w-8 h-8 text-red-500" />,
    content: (
      <div className="space-y-4">
        <p className="text-sm text-zinc-600 dark:text-zinc-400">
          使用 CrossChat 时，请注意以下安全事项：
        </p>
        <div className="space-y-2">
          <div className="flex items-start gap-2">
            <div className="w-2 h-2 bg-red-500 rounded-full mt-1.5 flex-shrink-0"></div>
            <p className="text-xs text-zinc-600 dark:text-zinc-400">
              不要在对话中分享敏感信息（密码、API密钥等）
            </p>
          </div>
          <div className="flex items-start gap-2">
            <div className="w-2 h-2 bg-red-500 rounded-full mt-1.5 flex-shrink-0"></div>
            <p className="text-xs text-zinc-600 dark:text-zinc-400">
              谨慎执行AI生成的代码，特别是系统命令
            </p>
          </div>
          <div className="flex items-start gap-2">
            <div className="w-2 h-2 bg-red-500 rounded-full mt-1.5 flex-shrink-0"></div>
            <p className="text-xs text-zinc-600 dark:text-zinc-400">
              定期备份重要的对话和配置
            </p>
          </div>
        </div>
        <div className="bg-gradient-to-r from-amber-50 to-yellow-50 dark:from-amber-900/20 dark:to-yellow-900/20 p-3 rounded-lg border border-amber-200/50 dark:border-amber-800/50">
          <p className="text-xs text-amber-700 dark:text-amber-300">
            提示：您可以随时在设置中重新查看本指引。
          </p>
        </div>
      </div>
    ),
  },
];

export default function WelcomeDialog() {
  const [open, setOpen] = useState(false);
  const [currentStep, setCurrentStep] = useState(0);

  useEffect(() => {
    // 检查是否是首次运行
    const hasSeenWelcome = localStorage.getItem("crosschat_welcome_seen");
    if (!hasSeenWelcome) {
      setOpen(true);
    }
  }, []);

  const handleNext = () => {
    if (currentStep < WELCOME_STEPS.length - 1) {
      setCurrentStep(currentStep + 1);
    }
  };

  const handlePrevious = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  const handleFinish = () => {
    localStorage.setItem("crosschat_welcome_seen", "true");
    setOpen(false);
    setCurrentStep(0);
  };

  const handleSkip = () => {
    localStorage.setItem("crosschat_welcome_seen", "true");
    setOpen(false);
    setCurrentStep(0);
  };

  const step = WELCOME_STEPS[currentStep];

  return (
    <Dialog.Root open={open} onOpenChange={setOpen}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/40 backdrop-blur-sm z-50 animate-in fade-in duration-200" />
        <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-white dark:bg-zinc-900 rounded-2xl shadow-2xl w-[480px] z-50 border border-zinc-200/70 dark:border-zinc-700/70 animate-in fade-in zoom-in-95 duration-200">
          {/* Header with gradient */}
          <div className="flex items-center justify-between px-5 py-3.5 border-b border-zinc-200/70 dark:border-zinc-700/70 bg-gradient-to-r from-purple-50/50 to-blue-50/50 dark:from-purple-950/20 dark:to-blue-950/20">
            <div className="flex items-center gap-3">
              {step.icon}
              <div>
                <Dialog.Title className="text-sm font-semibold bg-gradient-to-r from-purple-600 to-blue-600 bg-clip-text text-transparent">
                  {step.title}
                </Dialog.Title>
                <p className="text-xs text-zinc-500 dark:text-zinc-400">
                  {step.description}
                </p>
              </div>
            </div>
            <button
              onClick={handleSkip}
              className="p-1 rounded-lg text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 hover:bg-white/80 dark:hover:bg-zinc-800/80 transition-all duration-200"
              title="跳过指引"
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          {/* Content */}
          <div className="px-5 py-4 min-h-[200px]">
            {step.content}
          </div>

          {/* Progress with gradient */}
          <div className="px-5 py-2">
            <div className="flex justify-center gap-2">
              {WELCOME_STEPS.map((_, index) => (
                <div
                  key={index}
                  className={`w-2 h-2 rounded-full transition-all duration-200 ${
                    index === currentStep
                      ? "bg-gradient-to-r from-purple-500 to-blue-500 w-6"
                      : index < currentStep
                      ? "bg-green-500"
                      : "bg-zinc-300 dark:bg-zinc-600"
                  }`}
                />
              ))}
            </div>
          </div>

          {/* Footer */}
          <div className="flex items-center justify-between px-5 py-3.5 border-t border-zinc-200/70 dark:border-zinc-700/70 bg-zinc-50/50 dark:bg-zinc-900/50">
            <button
              onClick={handlePrevious}
              disabled={currentStep === 0}
              className="flex items-center gap-1 px-3 py-1.5 text-xs font-medium text-zinc-600 dark:text-zinc-400 hover:text-purple-600 dark:hover:text-purple-400 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              <ChevronLeft className="w-3.5 h-3.5" />
              上一步
            </button>

            <div className="flex items-center gap-2">
              <button
                onClick={handleSkip}
                className="px-3 py-1.5 text-xs font-medium text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 transition-colors"
              >
                跳过
              </button>

              {currentStep === WELCOME_STEPS.length - 1 ? (
                <button
                  onClick={handleFinish}
                  className="flex items-center gap-1 px-4 py-1.5 text-xs font-medium bg-gradient-to-r from-purple-500 to-blue-500 text-white rounded-lg hover:from-purple-600 hover:to-blue-600 transition-all duration-200 shadow-md shadow-purple-500/30 hover:shadow-lg hover:shadow-purple-500/40"
                >
                  <Check className="w-3.5 h-3.5" />
                  开始使用
                </button>
              ) : (
                <button
                  onClick={handleNext}
                  className="flex items-center gap-1 px-4 py-1.5 text-xs font-medium bg-gradient-to-r from-purple-500 to-blue-500 text-white rounded-lg hover:from-purple-600 hover:to-blue-600 transition-all duration-200 shadow-md shadow-purple-500/30 hover:shadow-lg hover:shadow-purple-500/40"
                >
                  下一步
                  <ChevronRight className="w-3.5 h-3.5" />
                </button>
              )}
            </div>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
