//! 模型下载工具

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const MODEL_URL: &str = "https://github.com/opencv/opencv_3rdparty/raw/dnn_samples_face_detector_20170830/res10_300x300_ssd_iter_140000.caffemodel";
const CONFIG_URL: &str = "https://raw.githubusercontent.com/opencv/opencv/master/samples/dnn/face_detector/deploy.prototxt";

fn main() -> Result<()> {
    println!("RTSP 人脸识别 - 模型下载工具\n");

    let model_dir = dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join("rtsp-face-recognition/models");

    fs::create_dir_all(&model_dir)?;
    println!("模型目录: {:?}", model_dir);

    let model_path = model_dir.join("res10_300x300_ssd_iter_140000.caffemodel");
    let config_path = model_dir.join("deploy.prototxt");

    // 下载模型
    if !model_path.exists() {
        println!("\n[1/2] 下载人脸检测模型...");
        download_file(MODEL_URL, &model_path)?;
    } else {
        println!("\n[1/2] 模型文件已存在，跳过下载");
    }

    // 下载配置
    if !config_path.exists() {
        println!("\n[2/2] 下载配置文件...");
        download_file(CONFIG_URL, &config_path)?;
    } else {
        println!("\n[2/2] 配置文件已存在，跳过下载");
    }

    println!("\n✓ 所有文件下载完成！");
    println!("  模型: {:?}", model_path);
    println!("  配置: {:?}", config_path);

    Ok(())
}

fn download_file(url: &str, path: &Path) -> Result<()> {
    let response = reqwest::blocking::get(url)?;
    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars(">#-"),
    );

    let mut file = fs::File::create(path)?;
    let mut downloaded = 0u64;

    for chunk in response.bytes()?.chunks(8192) {
        file.write_all(chunk)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("完成");
    Ok(())
}
