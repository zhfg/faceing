//! 人脸识别模块 - 使用 Tract ONNX 纯 Rust 推理引擎

use anyhow::{Context, Result};
use image::{imageops, DynamicImage, RgbImage};
use ndarray::{Array, Array4, Axis};
use opencv::core::Mat;
use std::path::Path;
use tract_onnx::prelude::*;
use tracing::info;

/// 人脸识别器
pub struct FaceRecognizer {
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
    input_size: (usize, usize),
}

/// 人脸特征向量
pub type FaceEmbedding = Vec<f32>;

impl FaceRecognizer {
    /// 创建新的人脸识别器
    pub fn new(model_path: &Path) -> Result<Self> {
        info!("加载人脸识别模型: {:?}", model_path);

        // 加载 ONNX 模型
        let model = tract_onnx::onnx()
            .model_for_path(model_path)?
            .into_optimized()?
            .into_runnable()?;

        info!("模型加载完成");

        Ok(Self {
            model,
            input_size: (112, 112),
        })
    }

    /// 使用默认模型路径
    pub fn with_default_model() -> Result<Self> {
        let model_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("rtsp-face-recognition/models");

        let model_candidates = [
            "mobilefacenet.onnx",
            "face_recognition_slim.onnx",
            "arcface.onnx",
        ];

        for model_name in &model_candidates {
            let model_path = model_dir.join(model_name);
            if model_path.exists() {
                return Self::new(&model_path);
            }
        }

        anyhow::bail!(
            "未找到人脸识别模型。请下载模型到: {:?}\n\
             支持模型: mobilefacenet.onnx, arcface.onnx, face_recognition_slim.onnx",
            model_dir
        )
    }

    /// 从 OpenCV Mat 提取人脸特征
    pub fn extract_embedding(&self,
        face_mat: &Mat) -> Result<FaceEmbedding> {
        // 将 Mat 转换为 RGB 图像
        let rgb_mat = Self::mat_to_rgb(face_mat)?;
        let image = Self::mat_to_image(&rgb_mat)?;

        // 预处理并推理
        let input = self.preprocess(&image)?;
        let result = self.model.run(tvec!(input.into()))?;

        // 提取特征向量
        let embedding = self.postprocess(&result)?;

        Ok(embedding)
    }

    /// 预处理图像
    fn preprocess(
        &self,
        image: &DynamicImage) -> Result<Tensor> {
        // 调整尺寸
        let resized = image.resize_exact(
            self.input_size.0 as u32,
            self.input_size.1 as u32,
            imageops::FilterType::Triangle,
        );

        // 转换为 RGB 数组
        let rgb = resized.to_rgb8();
        let raw = rgb.into_raw();

        // 归一化 (0-255 -> -1~1)
        let normalized: Vec<f32> = raw
            .iter()
            .map(|&p| (p as f32 - 127.5) / 127.5)
            .collect();

        // 构建输入张量 [1, 3, H, W]
        let array = Array4::from_shape_vec(
            (1, 3, self.input_size.1, self.input_size.0),
            normalized,
        )?;

        Ok(array.into_tensor())
    }

    /// 后处理输出
    fn postprocess(
        &self,
        outputs: &[Arc<Tensor>]) -> Result<FaceEmbedding> {
        if outputs.is_empty() {
            anyhow::bail!("模型输出为空");
        }

        // 获取输出张量
        let output = outputs[0].to_array_view::<f32>()?;

        // 展平为向量
        let embedding: Vec<f32> = output.iter().copied().collect();

        // L2 归一化
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            let normalized: Vec<f32> = embedding.iter().map(|x| x / norm).collect();
            Ok(normalized)
        } else {
            Ok(embedding)
        }
    }

    /// 将 Mat 转换为 RGB 格式
    fn mat_to_rgb(mat: &Mat) -> Result<Mat> {
        use opencv::imgproc;

        let mut rgb = Mat::default();
        imgproc::cvt_color(mat, &mut rgb, imgproc::COLOR_BGR2RGB, 0)?;
        Ok(rgb)
    }

    /// 将 Mat 转换为 image::DynamicImage
    fn mat_to_image(mat: &Mat) -> Result<DynamicImage> {
        let width = mat.cols() as u32;
        let height = mat.rows() as u32;

        // 获取数据
        let data = mat.data_bytes()?;

        // 创建 RGB 图像
        let rgb_image = RgbImage::from_raw(width, height, data.to_vec())
            .context("无法创建图像")?;

        Ok(DynamicImage::ImageRgb8(rgb_image))
    }

    /// 计算两个特征的相似度 (余弦相似度)
    pub fn similarity(a: &[f32], b: &[f32]) -> f32 {
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

    /// 计算欧氏距离
    pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }
}

/// 下载人脸识别模型
pub fn download_recognition_model() -> Result<()> {
    use indicatif::{ProgressBar, ProgressStyle};
    use std::io::Write;

    let model_dir = dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join("rtsp-face-recognition/models");

    std::fs::create_dir_all(&model_dir)?;

    // 使用 SFace 模型 (OpenCV Zoo)
    let model_url = "https://github.com/opencv/opencv_zoo/raw/main/models/face_recognition_sface/face_recognition_sface_2021dec.onnx";
    let model_path = model_dir.join("face_recognition_slim.onnx");

    if model_path.exists() {
        println!("人脸识别模型已存在");
        return Ok(());
    }

    println!("正在下载人脸识别模型...");

    let response = reqwest::blocking::get(model_url)?;
    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("=>-"),
    );

    let mut file = std::fs::File::create(&model_path)?;
    let mut downloaded = 0u64;

    for chunk in response.bytes()?.chunks(8192) {
        file.write_all(chunk)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("下载完成");
    info!("模型已保存到: {:?}", model_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((FaceRecognizer::similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(FaceRecognizer::similarity(&a, &c).abs() < 0.001);
    }
}
