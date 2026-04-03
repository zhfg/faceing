#!/bin/bash
# Linux 本地构建脚本

set -e

echo "==================================="
echo "RTSP 人脸识别系统 - Linux 构建脚本"
echo "==================================="

# 检查依赖
check_dep() {
    if command -v $1 &> /dev/null; then
        echo "✓ 找到 $1"
    else
        echo "✗ 未找到 $1"
        exit 1
    fi
}

echo ""
echo "[1/3] 检查依赖..."
check_dep rustc
check_dep cargo
check_dep pkg-config

# 检查 OpenCV
echo ""
echo "[2/3] 检查 OpenCV..."
if pkg-config --exists opencv4; then
    echo "✓ 找到 OpenCV $(pkg-config --modversion opencv4)"
else
    echo "✗ 未找到 OpenCV"
    echo "请安装: sudo apt-get install libopencv-dev"
    exit 1
fi

# 下载模型
echo ""
echo "[3/3] 下载模型文件..."
cargo run --bin download_models 2>/dev/null || true

# 编译
echo ""
echo "开始编译..."
cargo build --release

echo ""
echo "==================================="
echo "✓ 构建完成！"
echo "==================================="
echo ""
echo "可执行文件:"
ls -lh target/release/{rtsp_face_recognition,face_detection} 2>/dev/null || true
echo ""
echo "运行方式:"
echo "  ./target/release/rtsp_face_recognition rtsp://admin:password@192.168.1.100:554/stream"
echo "  ./target/release/face_detection rtsp://192.168.1.100:554/stream"
