import { motion } from "framer-motion";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

interface Props {
  text: string;
  isStreaming: boolean;
}

export default function StreamingText({ text, isStreaming }: Props) {
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className={`prose prose-sm dark:prose-invert max-w-none
        [&_pre]:overflow-x-auto [&_code]:break-all [&_a]:break-all
        ${isStreaming ? "streaming-cursor" : ""}`}
    >
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{text}</ReactMarkdown>
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
