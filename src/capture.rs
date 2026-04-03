//! RTSP 视频捕获模块

use anyhow::{Context, Result};
use opencv::{
    core::Mat,
    videoio::{VideoCapture, VideoCaptureTrait, CAP_FFMPEG},
};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// RTSP 视频捕获器
pub struct RtspCapture {
    capture: VideoCapture,
    url: String,
}

/// 帧数据
#[derive(Clone)]
pub struct FrameData {
    pub frame: Mat,
    pub timestamp: std::time::Instant,
}

impl RtspCapture {
    /// 创建新的 RTSP 捕获器
    pub fn new(url: &str) -> Result<Self> {
        // 设置环境变量使用 TCP 传输
        std::env::set_var("OPENCV_FFMPEG_CAPTURE_OPTIONS", "rtsp_transport;tcp");

        let capture = VideoCapture::from_file(url, CAP_FFMPEG)
            .with_context(|| format!("无法打开 RTSP 流: {}", url))?;

        if !capture.is_opened()? {
            anyhow::bail!("无法打开视频流");
        }

        let width = capture.get(opencv::videoio::CAP_PROP_FRAME_WIDTH)? as i32;
        let height = capture.get(opencv::videoio::CAP_PROP_FRAME_HEIGHT)? as i32;
        let fps = capture.get(opencv::videoio::CAP_PROP_FPS)?;

        info!("视频流信息: {}x{} @ {:.1} FPS", width, height, fps);

        Ok(Self {
            capture,
            url: url.to_string(),
        })
    }

    /// 读取一帧
    pub fn read_frame(&mut self) -> Result<Option<Mat>> {
        let mut frame = Mat::default();
        if self.capture.read(&mut frame)? {
            Ok(Some(frame))
        } else {
            Ok(None)
        }
    }

    /// 启动异步帧读取
    pub fn start_async(mut self, buffer_size: usize) -> mpsc::Receiver<FrameData> {
        let (tx, rx) = mpsc::channel::<FrameData>(buffer_size);

        tokio::spawn(async move {
            loop {
                match self.read_frame() {
                    Ok(Some(frame)) => {
                        let data = FrameData {
                            frame,
                            timestamp: std::time::Instant::now(),
                        };
                        if tx.send(data).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        warn!("读取帧失败，重试中...");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        error!("捕获错误: {}", e);
                        break;
                    }
                }
            }
        });

        rx
    }

    /// 获取视频宽度
    pub fn width(&self) -> Result<f64> {
        self.capture.get(opencv::videoio::CAP_PROP_FRAME_WIDTH)
    }

    /// 获取视频高度
    pub fn height(&self) -> Result<f64> {
        self.capture.get(opencv::videoio::CAP_PROP_FRAME_HEIGHT)
    }

    /// 获取 FPS
    pub fn fps(&self) -> Result<f64> {
        self.capture.get(opencv::videoio::CAP_PROP_FPS)
    }
}

impl Drop for RtspCapture {
    fn drop(&mut self) {
        let _ = self.capture.release();
    }
}
