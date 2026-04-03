//! RTSP 人脸识别系统主程序

use anyhow::Result;
use chrono::Local;
use opencv::{
    core::Rect,
    highgui, imgcodecs,
    prelude::*,
    types::Vector,
};
use rtsp_face_recognition::{
    capture::RtspCapture,
    database::FaceDatabase,
    detector::{ensure_models_exist, FaceDetector},
    draw_face_box, draw_stats, print_help,
    recognizer::{download_recognition_model, FaceEmbedding, FaceRecognizer},
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tracing::{info, warn};

/// 系统配置
struct Config {
    /// 识别间隔（帧）
    recognition_interval: u64,
    /// 人脸识别阈值
    recognition_threshold: f32,
    /// 数据目录
    data_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("rtsp-face-recognition");

        Self {
            recognition_interval: 5,
            recognition_threshold: 0.6,
            data_dir,
        }
    }
}

/// 应用程序状态
struct App {
    config: Config,
    detector: FaceDetector,
    recognizer: FaceRecognizer,
    database: FaceDatabase,
    db_path: PathBuf,
    captures_dir: PathBuf,
    stats: Stats,
    frame_count: u64,
}

#[derive(Default)]
struct Stats {
    start_time: Option<Instant>,
    total_frames: u64,
    faces_detected: u64,
}

impl Stats {
    fn fps(&self) -> f64 {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                return self.total_frames as f64 / elapsed;
            }
        }
        0.0
    }

    fn elapsed_secs(&self) -> f64 {
        self.start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }
}

/// 缓存的识别结果
struct CachedResult {
    rect: Rect,
    name: String,
    confidence: f32,
}

impl App {
    fn new() -> Result<Self> {
        let config = Config::default();

        // 创建目录
        std::fs::create_dir_all(&config.data_dir)?;
        let captures_dir = config.data_dir.join("captures");
        std::fs::create_dir_all(&captures_dir)?;

        // 加载或创建数据库
        let db_path = config.data_dir.join("face_database.json");
        let database = FaceDatabase::load(&db_path)?;

        // 确保模型存在并加载
        let _ = ensure_models_exist()?;
        let detector = FaceDetector::with_default_model()?;

        // 加载人脸识别模型
        let recognizer = FaceRecognizer::with_default_model()?;

        info!("系统初始化完成");

        Ok(Self {
            config,
            detector,
            recognizer,
            database,
            db_path,
            captures_dir,
            stats: Stats::default(),
            frame_count: 0,
        })
    }

    fn run(&mut self, rtsp_url: &str) -> Result<()> {
        print_help();

        // 连接 RTSP 流
        let mut capture = RtspCapture::new(rtsp_url)?;
        self.stats.start_time = Some(Instant::now());

        // 创建窗口
        highgui::named_window("RTSP Face Recognition", highgui::WINDOW_AUTOSIZE)?;

        let mut last_results: Vec<CachedResult> = Vec::new();

        loop {
            // 读取帧
            let Some(mut frame) = capture.read_frame()? else {
                warn!("读取帧失败，重试中...");
                std::thread::sleep(Duration::from_millis(100));
                continue;
            };

            self.stats.total_frames += 1;
            self.frame_count += 1;

            // 按间隔进行人脸检测和识别
            let should_recognize =
                self.frame_count % self.config.recognition_interval == 0;

            if should_recognize {
                last_results = self.process_frame(&frame)?;
            }

            // 绘制结果
            for result in &last_results {
                draw_face_box(
                    &mut frame,
                    &result.rect,
                    &result.name,
                    Some(result.confidence),
                )?;
            }

            // 绘制统计信息
            draw_stats(
                &mut frame,
                self.stats.fps(),
                self.stats.total_frames,
                last_results.len(),
                self.database.len(),
            )?;

            // 显示
            highgui::imshow("RTSP Face Recognition", &frame)?;

            // 处理键盘输入
            let key = highgui::wait_key(1)?;
            match key {
                113 | 81 => break,                   // Q/q
                115 | 83 => self.capture(&frame)?,   // S/s
                114 | 82 => self.handle_register(&frame)?, // R/r
                108 | 76 => self.list_faces(),       // L/l
                100 | 68 => self.handle_delete()?,   // D/d
                104 | 72 => print_help(),            // H/h
                _ => {}
            }
        }

        self.print_stats();
        Ok(())
    }

    /// 处理单帧，返回识别结果
    fn process_frame(&mut self, frame: &Mat) -> Result<Vec<CachedResult>> {
        let faces = self.detector.detect(frame)?;

        if !faces.is_empty() {
            self.stats.faces_detected += 1;
        }

        let mut results = Vec::new();

        for face in faces {
            // 提取人脸区域
            let face_roi = Mat::roi(frame, face.rect)?;

            // 提取特征
            let embedding = match self.recognizer.extract_embedding(&face_roi) {
                Ok(emb) => emb,
                Err(e) => {
                    warn!("特征提取失败: {}", e);
                    continue;
                }
            };

            // 识别人脸
            let (name, confidence) = if !self.database.is_empty() {
                self.recognize_face(&embedding)
                    .unwrap_or_else(|| ("未知".to_string(), 0.0))
            } else {
                ("未知".to_string(), 0.0)
            };

            results.push(CachedResult {
                rect: face.rect,
                name,
                confidence,
            });
        }

        Ok(results)
    }

    /// 在数据库中识别人脸
    fn recognize_face(&self, embedding: &FaceEmbedding) -> Option<(String, f32)> {
        let mut best_match: Option<(String, f32)> = None;
        let mut best_similarity = 0.0f32;

        for name in self.database.list_faces() {
            if let Some(samples) = self.database.get_faces(name) {
                for sample in samples {
                    let similarity = FaceRecognizer::similarity(embedding, sample);
                    if similarity > best_similarity && similarity > self.config.recognition_threshold {
                        best_similarity = similarity;
                        best_match = Some((name.to_string(), similarity));
                    }
                }
            }
        }

        best_match
    }

    fn capture(&self, frame: &Mat) -> Result<()> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("capture_{}.jpg", timestamp);
        let path = self.captures_dir.join(&filename);

        imgcodecs::imwrite(path.to_str().unwrap(), frame, &Vector::new())?;

        info!("截图已保存: {:?}", path);
        println!("[截图] 已保存: {:?}", path);

        Ok(())
    }

    fn handle_register(&mut self, frame: &Mat) -> Result<()> {
        // 检测人脸
        let faces = self.detector.detect(frame)?;

        if faces.is_empty() {
            println!("[注册] 错误: 未检测到人脸");
            return Ok(());
        }

        if faces.len() > 1 {
            println!("[注册] 错误: 检测到多个人脸，请确保画面中只有一个人");
            return Ok(());
        }

        let face = &faces[0];

        // 提取特征
        let face_roi = Mat::roi(frame, face.rect)?;
        let embedding = match self.recognizer.extract_embedding(&face_roi) {
            Ok(emb) => emb,
            Err(e) => {
                println!("[注册] 特征提取失败: {}", e);
                return Ok(());
            }
        };

        println!("\n[注册] 请输入姓名:");
        let mut name = String::new();
        std::io::stdin().read_line(&mut name)?;
        let name = name.trim();

        if name.is_empty() {
            println!("[注册] 取消注册");
            return Ok(());
        }

        // 保存截图
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("register_{}_{}.jpg", name, timestamp);
        let path = self.captures_dir.join(&filename);
        imgcodecs::imwrite(path.to_str().unwrap(), frame, &Vector::new())?;

        // 添加到数据库
        self.database.add_face(name, embedding);
        self.database.save(&self.db_path)?;

        println!("[注册] 已成功注册: {}", name);
        info!("已注册新人脸: {}", name);

        Ok(())
    }

    fn list_faces(&self) {
        let faces = self.database.list_faces();
        println!("\n[数据库] 已注册人脸列表:");

        if faces.is_empty() {
            println!("  (空)");
        } else {
            for (i, name) in faces.iter().enumerate() {
                let count = self.database.sample_count(name);
                println!("  {}. {} ({} 个样本)", i + 1, name, count);
            }
        }
        println!();
    }

    fn handle_delete(&mut self) -> Result<()> {
        let faces = self.database.list_faces();

        if faces.is_empty() {
            println!("[删除] 数据库为空");
            return Ok(());
        }

        println!("\n[删除] 请输入要删除的姓名:");
        for (i, name) in faces.iter().enumerate() {
            println!("  {}. {}", i + 1, name);
        }

        let mut name = String::new();
        std::io::stdin().read_line(&mut name)?;
        let name = name.trim();

        if self.database.remove_face(name) {
            self.database.save(&self.db_path)?;
            println!("[删除] 已删除: {}", name);
            info!("已删除人脸: {}", name);
        } else {
            println!("[删除] 未找到: {}", name);
        }

        Ok(())
    }

    fn print_stats(&self) {
        println!("\n[统计] 运行时间: {:.1}秒", self.stats.elapsed_secs());
        println!("[统计] 总帧数: {}", self.stats.total_frames);
        println!("[统计] 检测到人脸: {}次", self.stats.faces_detected);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 解析参数
    let args: Vec<String> = std::env::args().collect();
    let rtsp_url = if args.len() > 1 {
        args[1].clone()
    } else {
        println!("RTSP 人脸识别系统 (Rust版)");
        println!("用法: rtsp_face_recognition <RTSP_URL>\n");
        println!("示例:");
        println!("  rtsp_face_recognition rtsp://admin:password@192.168.1.100:554/stream");
        println!("\n请输入 RTSP URL:");
        let mut url = String::new();
        std::io::stdin().read_line(&mut url)?;
        url.trim().to_string()
    };

    if rtsp_url.is_empty() {
        anyhow::bail!("错误: 未提供 RTSP URL");
    }

    // 下载识别模型（如果不存在）
    if let Err(e) = download_recognition_model() {
        warn!("模型下载可能失败: {}", e);
    }

    // 运行应用
    let mut app = App::new()?;
    app.run(&rtsp_url)?;

    Ok(())
}
