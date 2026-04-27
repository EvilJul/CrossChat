/// SSE (Server-Sent Events) 增量解析器
/// 处理 bytes_stream 中的数据，按 "\n\n" 分割事件行
#[allow(dead_code)]
pub struct SseParser {
    buffer: String,
}

#[allow(dead_code)]
impl SseParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// 喂入新数据块，返回可解析的完整行列表
    pub fn feed(&mut self, data: &str) -> Vec<Vec<String>> {
        self.buffer.push_str(data);
        let mut events = Vec::new();

        // 按 "\n\n" 分割事件
        while let Some(pos) = self.buffer.find("\n\n") {
            let event_block = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 2..].to_string();

            // 按 "\n" 分割每一行
            let lines: Vec<String> = event_block
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();

            if !lines.is_empty() {
                events.push(lines);
            }
        }

        events
    }

    /// 获取缓冲区剩余数据
    pub fn remaining(&self) -> &str {
        &self.buffer
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_parser_single_event() {
        let mut parser = SseParser::new();
        let data = "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n";
        let events = parser.feed(data);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0][0], "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}");
    }

    #[test]
    fn test_sse_parser_partial_data() {
        let mut parser = SseParser::new();
        let events1 = parser.feed("data: partial");
        assert_eq!(events1.len(), 0);
        assert_eq!(parser.remaining(), "data: partial");

        let events2 = parser.feed(" data\n\n");
        assert_eq!(events2.len(), 1);
        assert_eq!(parser.remaining(), "");
    }
}
