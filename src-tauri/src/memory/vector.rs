use serde::{Deserialize, Serialize};

/// 向量检索配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorConfig {
    pub model: String,
    pub dimension: usize,
}

/// 简化的向量检索（使用余弦相似度）
pub struct VectorSearch;

impl VectorSearch {
    /// 计算余弦相似度
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }

    /// 简单的文本向量化（TF-IDF 简化版）
    pub fn simple_vectorize(text: &str) -> Vec<f32> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut vec = vec![0.0; 128]; // 固定维度

        for (i, word) in words.iter().enumerate() {
            let hash = word.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
            let idx = (hash as usize) % vec.len();
            vec[idx] += 1.0;
        }

        // 归一化
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vec.iter_mut().for_each(|x| *x /= norm);
        }

        vec
    }

    /// 搜索最相似的文本
    pub fn search_similar(query: &str, candidates: &[(String, String)], limit: usize) -> Vec<(usize, f32)> {
        let query_vec = Self::simple_vectorize(query);
        let mut scores: Vec<(usize, f32)> = candidates
            .iter()
            .enumerate()
            .map(|(idx, (task, solution))| {
                let text = format!("{} {}", task, solution);
                let vec = Self::simple_vectorize(&text);
                let score = Self::cosine_similarity(&query_vec, &vec);
                (idx, score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores.into_iter().take(limit).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((VectorSearch::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        assert!((VectorSearch::cosine_similarity(&c, &d) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_simple_vectorize() {
        let vec = VectorSearch::simple_vectorize("hello world");
        assert_eq!(vec.len(), 128);

        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001); // 归一化
    }

    #[test]
    fn test_search_similar() {
        let candidates = vec![
            ("读取文件".to_string(), "使用 read_file 工具".to_string()),
            ("写入文件".to_string(), "使用 write_file 工具".to_string()),
            ("删除文件".to_string(), "使用 delete_file 工具".to_string()),
        ];

        let results = VectorSearch::search_similar("读取数据", &candidates, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 0); // "读取文件" 最相似
    }
}
