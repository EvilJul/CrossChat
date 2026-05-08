# 模型上下文长度修正报告

## 📋 修正说明

根据官方文档和最新信息，对国内主流模型的上下文长度进行了修正。

---

## 🔍 查证来源

### DeepSeek
**官方信息来源**:
- DeepSeek API 官方文档
- 多个技术文档确认

**修正内容**:
| 模型 | 修正前 | 修正后 | 依据 |
|------|--------|--------|------|
| deepseek-chat | 64K | **128K** | V3/V4 版本支持 128K 上下文 |
| deepseek-reasoner | 64K | **128K** | V3/V4 版本支持 128K 上下文 |
| deepseek-coder | 16K | **128K** | 统一为 128K |
| deepseek-v4 | - | **1M** | V4 新版本支持 1M 上下文 |

**关键信息**:
- DeepSeek V3/V4 模型支持 **128K tokens** 上下文窗口
- DeepSeek V4 最新版本支持 **1M tokens** 上下文
- deepseek-chat 和 deepseek-reasoner 都基于 V3/V4，支持 128K

---

### 通义千问 (Qwen)
**官方信息来源**:
- 阿里云百炼官方文档
- Qwen 官方博客
- Qwen2.5 系列技术报告

**修正内容**:
| 模型 | 修正前 | 修正后 | 依据 |
|------|--------|--------|------|
| qwen-max | 30K | **32K** | 官方文档明确：32K 上下文 |
| qwen-plus | 30K | **128K** | Qwen2.5 系列：128K 上下文 |
| qwen-turbo | 8K | **1M** | Qwen2.5-Turbo：1M 上下文 |
| qwen2.5 | 32K | **128K** | Qwen2.5 系列标准：128K |

**关键信息**:
- **qwen-max**: 32K tokens（官方文档确认）
- **qwen-plus**: 基于 Qwen2.5，支持 **128K tokens**
- **qwen-turbo**: Qwen2.5-Turbo 版本，支持 **1M tokens**（百万级上下文）
- **qwen-long**: 专门的长文本模型，支持 **1M tokens**

---

### Groq
**新增模型**:
| 模型 | 上下文长度 | 说明 |
|------|-----------|------|
| llama-3.1-70b-versatile | **128K** | Llama 3.1 支持长上下文 |

---

### Ollama
**新增模型**:
| 模型 | 上下文长度 | 说明 |
|------|-----------|------|
| llama3.1 | **128K** | Llama 3.1 版本 |
| llama3.2 | **128K** | Llama 3.2 版本 |
| qwen2.5 | **128K** | 本地 Qwen2.5 |

---

## 📊 完整的模型上下文长度表

### OpenAI 系列
| 模型 | 上下文长度 | 备注 |
|------|-----------|------|
| gpt-4o | 128K | 最新旗舰模型 |
| gpt-4o-mini | 128K | 轻量版 |
| gpt-4-turbo | 128K | Turbo 版本 |
| gpt-4 | 8K | 经典版本 |
| gpt-3.5-turbo | 16K | 经典模型 |
| o3-mini | 200K | 推理模型 |
| o1 | 200K | 推理模型 |
| o1-mini | 128K | 推理模型轻量版 |

### Anthropic Claude 系列
| 模型 | 上下文长度 | 备注 |
|------|-----------|------|
| claude-sonnet-4-6 | 200K | 最新 Sonnet |
| claude-opus-4-6 | 200K | 最新 Opus |
| claude-haiku-4-5 | 200K | 最新 Haiku |
| claude-3-5-sonnet | 200K | Claude 3.5 |
| claude-3-opus | 200K | Claude 3 |
| claude-3-sonnet | 200K | Claude 3 |
| claude-3-haiku | 200K | Claude 3 |

### DeepSeek 系列
| 模型 | 上下文长度 | 备注 |
|------|-----------|------|
| deepseek-chat | 128K | ✅ 修正：64K → 128K |
| deepseek-reasoner | 128K | ✅ 修正：64K → 128K |
| deepseek-coder | 128K | ✅ 修正：16K → 128K |
| deepseek-v3 | 128K | V3 版本 |
| deepseek-v4 | 1M | ✅ 新增：V4 百万级上下文 |

### 通义千问 (Qwen) 系列
| 模型 | 上下文长度 | 备注 |
|------|-----------|------|
| qwen-max | 32K | ✅ 修正：30K → 32K |
| qwen-plus | 128K | ✅ 修正：30K → 128K |
| qwen-turbo | 1M | ✅ 修正：8K → 1M |
| qwen2.5 | 128K | ✅ 修正：32K → 128K |
| qwen2 | 32K | Qwen2 系列 |
| qwen-long | 1M | ✅ 新增：长文本专用 |

### Groq 系列
| 模型 | 上下文长度 | 备注 |
|------|-----------|------|
| llama-3.3-70b-versatile | 8K | Llama 3.3 |
| llama-3.1-70b-versatile | 128K | ✅ 新增：Llama 3.1 |
| mixtral-8x7b-32768 | 32K | Mixtral |

### Ollama 本地模型
| 模型 | 上下文长度 | 备注 |
|------|-----------|------|
| llama3 | 8K | Llama 3.0 |
| llama3.1 | 128K | ✅ 新增：Llama 3.1 |
| llama3.2 | 128K | ✅ 新增：Llama 3.2 |
| codellama | 16K | Code Llama |
| qwen2.5 | 128K | ✅ 修正：32K → 128K |

---

## 🔧 技术实现

### 匹配逻辑
```typescript
function getModelContextLimit(modelName: string): number {
  // 1. 精确匹配
  if (MODEL_CONTEXT_LIMITS[modelName]) {
    return MODEL_CONTEXT_LIMITS[modelName];
  }
  
  // 2. 模糊匹配（处理版本号等变体）
  const lowerModel = modelName.toLowerCase();
  for (const [key, limit] of Object.entries(MODEL_CONTEXT_LIMITS)) {
    if (lowerModel.includes(key.toLowerCase()) || 
        key.toLowerCase().includes(lowerModel)) {
      return limit;
    }
  }
  
  // 3. 默认值：128K
  return 128000;
}
```

### 匹配示例
```typescript
// 精确匹配
getModelContextLimit("deepseek-chat")           // 128000
getModelContextLimit("qwen-turbo")              // 1000000

// 模糊匹配（处理版本号）
getModelContextLimit("deepseek-chat-v3")        // 128000
getModelContextLimit("qwen-turbo-2024-01-01")   // 1000000
getModelContextLimit("qwen2.5-72b-instruct")    // 128000

// 默认值
getModelContextLimit("unknown-model")           // 128000
```

---

## 📈 影响分析

### 用户体验改进
1. **更准确的上下文提示**
   - DeepSeek 用户：从 64K → 128K，可以处理更长的对话
   - Qwen-Turbo 用户：从 8K → 1M，可以处理超长文档
   - Qwen-Plus 用户：从 30K → 128K，显著提升

2. **百分比计算更合理**
   - 之前：使用 qwen-turbo 时，8K tokens 就显示 100%
   - 现在：使用 qwen-turbo 时，1M tokens 才显示 100%

3. **避免误导**
   - 之前：deepseek-chat 显示 64K，实际支持 128K
   - 现在：正确显示 128K

---

## 🧪 测试建议

### 功能测试
- [ ] 选择 deepseek-chat，确认显示 "/ 128K tokens"
- [ ] 选择 deepseek-reasoner，确认显示 "/ 128K tokens"
- [ ] 选择 qwen-max，确认显示 "/ 32K tokens"
- [ ] 选择 qwen-plus，确认显示 "/ 128K tokens"
- [ ] 选择 qwen-turbo，确认显示 "/ 1000K tokens"
- [ ] 选择 deepseek-v4，确认显示 "/ 1000K tokens"

### 模糊匹配测试
- [ ] 测试 "deepseek-chat-v3" 是否匹配到 128K
- [ ] 测试 "qwen2.5-72b-instruct" 是否匹配到 128K
- [ ] 测试 "qwen-turbo-latest" 是否匹配到 1M

### 百分比测试
- [ ] 发送 10K tokens 的对话到 qwen-turbo，确认显示约 1%
- [ ] 发送 10K tokens 的对话到 qwen-max，确认显示约 31%
- [ ] 发送 100K tokens 的对话到 deepseek-chat，确认显示约 78%

---

## 📝 数据来源

### DeepSeek
- 官方文档: https://api-docs.deepseek.com/
- 技术报告: https://www.datastudios.org/post/deepseek-context-window-*
- 确认时间: 2026-05-08

### 通义千问
- 官方文档: https://help.aliyun.com/zh/model-studio/
- 官方博客: https://qwenlm.github.io/zh/blog/qwen2.5-turbo/
- 确认时间: 2026-05-08

### 其他模型
- OpenAI: 官方文档
- Anthropic: 官方文档
- Groq: 官方文档
- Ollama: 社区文档

---

## ✅ 修正总结

### 主要修正
1. **DeepSeek 系列**: 64K → 128K（V3/V4 版本）
2. **qwen-max**: 30K → 32K（官方文档确认）
3. **qwen-plus**: 30K → 128K（Qwen2.5 系列）
4. **qwen-turbo**: 8K → 1M（Qwen2.5-Turbo）
5. **qwen2.5**: 32K → 128K（Qwen2.5 标准）

### 新增模型
1. **deepseek-v4**: 1M（最新版本）
2. **qwen-long**: 1M（长文本专用）
3. **llama-3.1-70b**: 128K（Groq）
4. **llama3.1/3.2**: 128K（Ollama）

### 影响范围
- ✅ 国内用户体验显著改善
- ✅ 上下文计算更准确
- ✅ 支持更多最新模型

---

**修正时间**: 2026-05-08  
**版本**: v1.2  
**状态**: ✅ 修正完成，等待测试
