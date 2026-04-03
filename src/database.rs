//! 人脸数据库模块

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use tracing::{info, warn};

/// 人脸特征向量（128维）
pub type FaceEmbedding = Vec<f32>;

/// 人脸数据库
#[derive(Debug, Serialize, Deserialize)]
pub struct FaceDatabase {
    /// name -> list of embeddings
    faces: HashMap<String, Vec<FaceEmbedding>>,
    version: u32,
}

impl Default for FaceDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl FaceDatabase {
    /// 创建空数据库
    pub fn new() -> Self {
        Self {
            faces: HashMap::new(),
            version: 1,
        }
    }

    /// 从文件加载数据库
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let db: FaceDatabase = serde_json::from_str(&content)?;
            info!("加载了 {} 个人脸", db.faces.len());
            Ok(db)
        } else {
            Ok(Self::new())
        }
    }

    /// 保存数据库到文件
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// 添加人脸
    pub fn add_face(&mut self, name: &str, embedding: FaceEmbedding) {
        self.faces
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(embedding);
    }

    /// 获取所有人脸名称
    pub fn list_faces(&self) -> Vec<&String> {
        self.faces.keys().collect()
    }

    /// 删除人脸
    pub fn remove_face(&mut self, name: &str) -> bool {
        self.faces.remove(name).is_some()
    }

    /// 获取某个人脸的所有样本
    pub fn get_faces(&self, name: &str) -> Option<&Vec<FaceEmbedding>> {
        self.faces.get(name)
    }

    /// 识别人脸 - 返回最匹配的名称和相似度
    pub fn recognize(&self, embedding: &[f32], threshold: f32) -> Option<(String, f32)> {
        let mut best_match: Option<(String, f32)> = None;
        let mut best_distance = f32::MAX;

        for (name, embeddings) in &self.faces {
            for stored in embeddings {
                let distance = euclidean_distance(embedding, stored);
                if distance < best_distance && distance < threshold {
                    best_distance = distance;
                    best_match = Some((name.clone(), 1.0 - distance));
                }
            }
        }

        best_match
    }

    /// 获取某个人脸的样本数量
    pub fn sample_count(&self, name: &str) -> usize {
        self.faces.get(name).map(|v| v.len()).unwrap_or(0)
    }

    /// 数据库是否为空
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    /// 人脸数量
    pub fn len(&self) -> usize {
        self.faces.len()
    }
}

/// 计算欧氏距离
fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

/// 计算余弦相似度
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database() {
        let mut db = FaceDatabase::new();
        db.add_face("test", vec![0.1; 128]);
        assert_eq!(db.len(), 1);
        assert!(db.list_faces().contains(&&"test".to_string()));
    }

    #[test]
    fn test_recognize() {
        let mut db = FaceDatabase::new();
        db.add_face("alice", vec![0.1; 128]);
        db.add_face("bob", vec![0.9; 128]);

        let result = db.recognize(&vec![0.11; 128], 0.5);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "alice");
    }
}
