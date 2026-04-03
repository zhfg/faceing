//! RTSP 人脸识别系统库

pub mod capture;
pub mod database;
pub mod detector;
pub mod recognizer;

use anyhow::Result;
use opencv::{
    core::{Point, Rect, Scalar},
    highgui,
    imgproc,
    prelude::*,
};

/// 在图像上绘制人脸框
pub fn draw_face_box(
    frame: &mut Mat,
    rect: &Rect,
    label: &str,
    confidence: Option<f32>,
) -> Result<()> {
    // 选择颜色：已识别为绿色，未识别为红色
    let color = if label == "未知" {
        Scalar::new(0.0, 0.0, 255.0, 0.0)
    } else {
        Scalar::new(0.0, 255.0, 0.0, 0.0)
    };

    // 绘制矩形框
    imgproc::rectangle(frame, *rect, color, 2, imgproc::LINE_8, 0)?;

    // 准备标签文本
    let text = if let Some(conf) = confidence {
        format!("{} ({:.1}%)", label, conf * 100.0)
    } else {
        label.to_string()
    };

    // 计算文本大小
    let baseline = 0;
    let text_size = imgproc::get_text_size(
        &text,
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.6,
        2,
        &baseline,
    )?;

    // 绘制标签背景
    let label_rect = Rect::new(
        rect.x,
        rect.y - text_size.height - 10,
        text_size.width,
        text_size.height + 10,
    );
    imgproc::rectangle(frame, label_rect, color, -1, imgproc::LINE_8, 0)?;

    // 绘制文本
    imgproc::put_text(
        frame,
        &text,
        Point::new(rect.x, rect.y - 5),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.6,
        Scalar::new(255.0, 255.0, 255.0, 0.0),
        2,
        imgproc::LINE_8,
        false,
    )?;

    Ok(())
}

/// 在图像上绘制统计信息
pub fn draw_stats(
    frame: &mut Mat,
    fps: f64,
    total_frames: u64,
    face_count: usize,
    db_size: usize,
) -> Result<()> {
    let stats = vec![
        format!("FPS: {:.1}", fps),
        format!("Frames: {}", total_frames),
        format!("Faces: {}", face_count),
        format!("Database: {}", db_size),
    ];

    let mut y = 30;
    for text in stats {
        imgproc::put_text(
            frame,
            &text,
            Point::new(10, y),
            imgproc::FONT_HERSHEY_SIMPLEX,
            0.6,
            Scalar::new(0.0, 255.0, 255.0, 0.0),
            2,
            imgproc::LINE_8,
            false,
        )?;
        y += 25;
    }

    Ok(())
}

/// 显示帮助信息
pub fn print_help() {
    println!("\n{}", "=".repeat(50));
    println!("操作说明:");
    println!("  Q - 退出程序");
    println!("  S - 截图保存");
    println!("  R - 注册新人脸 (输入姓名)");
    println!("  L - 列出已注册人脸");
    println!("  D - 删除人脸");
    println!("  H - 显示帮助");
    println!("{}", "=".repeat(50));
}
