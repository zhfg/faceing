//! 人脸检测模块 - 使用 OpenCV DNN 深度学习模型

use anyhow::{Context, Result};
use opencv::{
    core::{self, Mat, Rect, Scalar, Size, Vector},
    dnn::{self, Net},
    prelude::*,
};
use std::path::Path;

/// 人脸检测器（基于 OpenCV DNN）
pub struct FaceDetector {
    net: Net,
    conf_threshold: f32,
    nms_threshold: f32,
    input_size: Size,
}

/// 检测到的人脸
#[derive(Debug, Clone)]
pub struct DetectedFace {
    pub rect: Rect,
    pub confidence: f32,
}

impl FaceDetector {
    /// 创建新的人脸检测器
    pub fn new(model_path: &Path, config_path: &Path) -> Result<Self> {
        let net = dnn::read_net_from_caffe(
            config_path.to_str().context("无效的配置路径")?,
            model_path.to_str().context("无效的模型路径")?,
        )?;

        Ok(Self {
            net,
            conf_threshold: 0.5,
            nms_threshold: 0.4,
            input_size: Size::new(300, 300),
        })
    }

    /// 使用默认模型路径创建检测器
    pub fn with_default_model() -> Result<Self> {
        let model_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("rtsp-face-recognition/models");

        let model_path = model_dir.join("res10_300x300_ssd_iter_140000.caffemodel");
        let config_path = model_dir.join("deploy.prototxt");

        if !model_path.exists() || !config_path.exists() {
            anyhow::bail!(
                "模型文件不存在。请运行: cargo run --bin download_models\n\
                 或手动下载到: {:?}",
                model_dir
            );
        }

        Self::new(&model_path, &config_path)
    }

    /// 检测人脸
    pub fn detect(&mut self, frame: &Mat) -> Result<Vec<DetectedFace>> {
        let (h, w) = (frame.rows(), frame.cols());

        // 预处理
        let blob = dnn::blob_from_image(
            frame,
            1.0,
            self.input_size,
            Scalar::new(104.0, 177.0, 123.0, 0.0),
            false,
            false,
            core::CV_32F,
        )?;

        // 前向传播
        self.net.set_input(&blob, "", 1.0, Scalar::default())?;
        let mut detections = Mat::default();
        self.net.forward(&mut detections, &Vector::new())?;

        // 解析结果
        let mut faces = Vec::new();
        let data = detections.data_typed::<f32>()?;
        let detection_size = detections.size()?;

        for i in 0..detection_size.width as usize {
            let offset = i * 7;
            let confidence = data[offset + 2];

            if confidence > self.conf_threshold {
                let x1 = (data[offset + 3] * w as f32) as i32;
                let y1 = (data[offset + 4] * h as f32) as i32;
                let x2 = (data[offset + 5] * w as f32) as i32;
                let y2 = (data[offset + 6] * h as f32) as i32;

                let rect = Rect::new(x1, y1, x2 - x1, y2 - y1);
                faces.push(DetectedFace { rect, confidence });
            }
        }

        Ok(faces)
    }

    /// 设置置信度阈值
    pub fn set_conf_threshold(&mut self, threshold: f32) {
        self.conf_threshold = threshold.clamp(0.0, 1.0);
    }
}

/// 下载模型文件的工具函数
pub fn ensure_models_exist() -> Result<(std::path::PathBuf, std::path::PathBuf)> {
    let model_dir = dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join("rtsp-face-recognition/models");

    std::fs::create_dir_all(&model_dir)?;

    let model_path = model_dir.join("res10_300x300_ssd_iter_140000.caffemodel");
    let config_path = model_dir.join("deploy.prototxt");

    // 模型下载链接
    const MODEL_URL: &str = "https://github.com/opencv/opencv_3rdparty/raw/dnn_samples_face_detector_20170830/res10_300x300_ssd_iter_140000.caffemodel";
    const CONFIG_URL: &str = "https://raw.githubusercontent.com/opencv/opencv/master/samples/dnn/face_detector/deploy.prototxt";

    // 下载模型文件
    if !model_path.exists() {
        println!("正在下载人脸检测模型...");
        download_file(MODEL_URL, &model_path)?;
        println!("模型下载完成");
    }

    if !config_path.exists() {
        println!("正在下载配置文件...");
        download_file(CONFIG_URL, &config_path)?;
        println!("配置下载完成");
    }

    Ok((model_path, config_path))
}

fn download_file(url: &str, path: &Path) -> Result<()> {
    use indicatif::{ProgressBar, ProgressStyle};
    use std::io::Write;

    let response = reqwest::blocking::get(url)?;
    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("#>-"),
    );

    let mut file = std::fs::File::create(path)?;
    let mut downloaded = 0u64;

    for chunk in response.bytes()?.chunks(8192) {
        file.write_all(chunk)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("下载完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_models() {
        let (model, config) = ensure_models_exist().unwrap();
        assert!(model.exists());
        assert!(config.exists());
    }
}
