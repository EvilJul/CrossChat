import { motion } from "framer-motion";

interface Props {
  text: string;
  isStreaming: boolean;
}

export default function StreamingText({ text, isStreaming }: Props) {
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className={isStreaming ? "streaming-cursor" : ""}
    >
      {/* 将换行符渲染为 <br /> */}
      {text.split("\n").map((line, i, arr) => (
        <span key={i}>
          {line}
          {i < arr.length - 1 && <br />}
        </span>
      ))}
      {/* 流式生成中的脉冲指示器 */}
      {isStreaming && (
        <motion.span
          animate={{ opacity: [1, 0.3, 1] }}
          transition={{ duration: 1.2, repeat: Infinity }}
          className="ml-0.5 inline-block w-1.5 h-4 bg-slate-400 rounded-sm align-middle"
        />
      )}
    </motion.div>
  );
}
