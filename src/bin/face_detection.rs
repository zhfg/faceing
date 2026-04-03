//! 简化版人脸检测 - 仅需 OpenCV

use anyhow::{Context, Result};
use opencv::{
    core::{Rect, Scalar, Size, Vector},
    highgui,
    imgproc::{self, CascadeClassifier},
    prelude::*,
    videoio::{VideoCapture, VideoCaptureTrait, CAP_FFMPEG},
};
use std::time::{Duration, Instant};
use tracing::{info, warn};

struct FaceDetector {
    classifier: CascadeClassifier,
}

impl FaceDetector {
    fn new() -> Result<Self> {
        let xml_path = opencv::core::find_file(
            "haarcascades/haarcascade_frontalface_default.xml",
            true,
            false,
        )?;
        let classifier = CascadeClassifier::new(&xml_path)?;
        Ok(Self { classifier })
    }

    fn detect(&mut self, frame: &Mat) -> Result<Vec<Rect>> {
        let mut gray = Mat::default();
        imgproc::cvt_color(frame, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;

        let mut faces = Vector::<Rect>::new();
        self.classifier.detect_multi_scale(
            &gray,
            &mut faces,
            1.1,      // scale factor
            5,        // min neighbors
            0,        // flags
            Size::new(30, 30),  // min size
            Size::default(),    // max size
        )?;

        Ok(faces.to_vec())
    }
}

struct RtspPlayer {
    capture: VideoCapture,
    detector: FaceDetector,
    stats: Stats,
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
}

impl RtspPlayer {
    fn new(rtsp_url: &str) -> Result<Self> {
        // 设置 RTSP 使用 TCP
        std::env::set_var("OPENCV_FFMPEG_CAPTURE_OPTIONS", "rtsp_transport;tcp");

        let capture = VideoCapture::from_file(rtsp_url, CAP_FFMPEG)
            .with_context(|| format!("无法打开 RTSP 流: {}", rtsp_url))?;

        if !capture.is_opened()? {
            anyhow::bail!("无法打开视频流");
        }

        let width = capture.get(opencv::videoio::CAP_PROP_FRAME_WIDTH)? as i32;
        let height = capture.get(opencv::videoio::CAP_PROP_FRAME_HEIGHT)? as i32;
        let fps = capture.get(opencv::videoio::CAP_PROP_FPS)?;

        info!("已连接到 RTSP 流");
        info!("分辨率: {}x{}, FPS: {:.1}", width, height, fps);

        let detector = FaceDetector::new()?;

        Ok(Self {
            capture,
            detector,
            stats: Stats::default(),
        })
    }

    fn run(&mut self) -> Result<()> {
        println!("\n{}", "=".repeat(40));
        println!("RTSP 人脸检测 (简化版)");
        println!("按 'Q' 退出");
        println!("{}\n", "=".repeat(40));

        self.stats.start_time = Some(Instant::now());

        highgui::named_window("Face Detection", highgui::WINDOW_AUTOSIZE)?;

        loop {
            let mut frame = Mat::default();
            if !self.capture.read(&mut frame)? {
                warn!("读取帧失败，重试中...");
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }

            self.stats.total_frames += 1;

            // 检测人脸
            let faces = match self.detector.detect(&frame) {
                Ok(f) => f,
                Err(e) => {
                    warn!("检测失败: {}", e);
                    continue;
                }
            };

            if !faces.is_empty() {
                self.stats.faces_detected += faces.len() as u64;
            }

            // 绘制人脸框
            for face in &faces {
                imgproc::rectangle(
                    &mut frame,
                    *face,
                    Scalar::new(0.0, 255.0, 0.0, 0.0),
                    2,
                    imgproc::LINE_8,
                    0,
                )?;
            }

            // 显示信息
            let text = format!(
                "Faces: {} | FPS: {:.1}",
                faces.len(),
                self.stats.fps()
            );
            imgproc::put_text(
                &mut frame,
                &text,
                opencv::core::Point::new(10, 30),
                imgproc::FONT_HERSHEY_SIMPLEX,
                0.7,
                Scalar::new(0.0, 255.0, 0.0, 0.0),
                2,
                imgproc::LINE_8,
                false,
            )?;

            highgui::imshow("Face Detection", &frame)?;

            if highgui::wait_key(1)? == 113 {
                // 'q'
                break;
            }
        }

        self.print_stats();
        Ok(())
    }

    fn print_stats(&self) {
        if let Some(start) = self.stats.start_time {
            let elapsed = start.elapsed().as_secs_f64();
            println!("\n[统计] 运行时间: {:.1}秒", elapsed);
            println!("[统计] 总帧数: {}", self.stats.total_frames);
            println!("[统计] 平均 FPS: {:.1}", self.stats.fps());
            println!("[统计] 检测到人脸: {}次", self.stats.faces_detected);
        }
    }
}

impl Drop for RtspPlayer {
    fn drop(&mut self) {
        let _ = self.capture.release();
        highgui::destroy_all_windows().ok();
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let rtsp_url = if args.len() > 1 {
        args[1].clone()
    } else {
        println!("RTSP 人脸检测 (简化版)");
        println!("用法: face_detection <RTSP_URL>\n");
        println!("示例:");
        println!("  face_detection rtsp://192.168.1.100:554/stream");
        println!("\n请输入 RTSP URL:");
        let mut url = String::new();
        std::io::stdin().read_line(&mut url)?;
        url.trim().to_string()
    };

    if rtsp_url.is_empty() {
        anyhow::bail!("错误: 未提供 RTSP URL");
    }

    let mut player = RtspPlayer::new(&rtsp_url)?;
    player.run()?;

    Ok(())
}
