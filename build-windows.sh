#!/bin/bash
# Windows 交叉编译脚本

set -e

echo "==================================="
echo "RTSP 人脸识别系统 - Windows 构建脚本"
echo "==================================="

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查依赖
check_dependency() {
    if ! command -v $1 &> /dev/null; then
        echo -e "${RED}错误: 未找到 $1${NC}"
        return 1
    fi
    echo -e "${GREEN}✓ 找到 $1${NC}"
    return 0
}

echo ""
echo "[1/5] 检查依赖..."
echo "-----------------------------------"

# 检查必要的依赖
MISSING_DEPS=0

if ! check_dependency "rustc"; then
    MISSING_DEPS=1
fi

if ! check_dependency "cargo"; then
    MISSING_DEPS=1
fi

if ! check_dependency "x86_64-w64-mingw32-gcc"; then
    echo -e "${YELLOW}⚠ 未找到 MinGW-w64 交叉编译器${NC}"
    echo "  请安装: sudo apt-get install mingw-w64"
    MISSING_DEPS=1
fi

if [ $MISSING_DEPS -eq 1 ]; then
    echo ""
    echo -e "${RED}请先安装缺失的依赖${NC}"
    exit 1
fi

# 添加 Windows 目标
echo ""
echo "[2/5] 添加 Windows 编译目标..."
echo "-----------------------------------"
rustup target add x86_64-pc-windows-gnu || true

# 检查并设置 OpenCV
echo ""
echo "[3/5] 配置 OpenCV..."
echo "-----------------------------------"

if [ -z "$OPENCV_LINK_LIBS" ]; then
    echo -e "${YELLOW}⚠ 未设置 OpenCV 环境变量${NC}"
    echo ""
    echo "Windows 编译需要 OpenCV Windows 库。请执行以下操作之一："
    echo ""
    echo "选项 1: 使用预编译的 OpenCV Windows 库"
    echo "  1. 下载: https://opencv.org/releases/"
    echo "  2. 解压到 /opt/opencv-win"
    echo "  3. 设置环境变量："
    echo "     export OPENCV_LINK_PATHS=/opt/opencv-win/x64/mingw/lib"
    echo "     export OPENCV_INCLUDE_PATHS=/opt/opencv-win/include"
    echo "     export OPENCV_LINK_LIBS=opencv_world490,opencv_videoio490,..."
    echo ""
    echo "选项 2: 使用 OpenCV 的 pkg-config"
    echo "  如果已安装 mingw 版本的 OpenCV:"
    echo "     export PKG_CONFIG_PATH=/usr/x86_64-w64-mingw32/lib/pkgconfig"
    echo ""
fi

# 创建输出目录
echo ""
echo "[4/5] 创建输出目录..."
echo "-----------------------------------"
mkdir -p dist/windows
echo -e "${GREEN}✓ 创建 dist/windows${NC}"

# 构建
echo ""
echo "[5/5] 开始编译..."
echo "-----------------------------------"

# 设置 Windows 编译环境
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++
export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar

# 编译主程序
echo "编译 rtsp_face_recognition.exe..."
cargo build --release --target x86_64-pc-windows-gnu --bin rtsp_face_recognition 2>&1 | tee build.log

if [ ${PIPESTATUS[0]} -eq 0 ]; then
    cp target/x86_64-pc-windows-gnu/release/rtsp_face_recognition.exe dist/windows/
    echo -e "${GREEN}✓ 编译成功: dist/windows/rtsp_face_recognition.exe${NC}"
else
    echo -e "${RED}✗ 编译失败${NC}"
    echo ""
    echo "常见错误及解决方案："
    echo "1. OpenCV 未找到 - 确保设置了正确的 OPENCV_* 环境变量"
    echo "2. 链接错误 - 可能需要手动指定库路径"
    echo "3. 查看 build.log 获取详细信息"
    exit 1
fi

# 编译简化版
echo "编译 face_detection.exe..."
cargo build --release --target x86_64-pc-windows-gnu --bin face_detection 2>&1 | tee -a build.log

if [ ${PIPESTATUS[0]} -eq 0 ]; then
    cp target/x86_64-pc-windows-gnu/release/face_detection.exe dist/windows/
    echo -e "${GREEN}✓ 编译成功: dist/windows/face_detection.exe${NC}"
fi

# 复制说明文件
cat > dist/windows/README.txt << 'EOF'
RTSP 人脸识别系统 - Windows 版本
====================================

系统要求：
- Windows 10/11 (64位)
- Visual C++ Redistributable 2019+ 或已安装 MinGW 运行时
- 网络摄像头或 RTSP 视频流

使用方法：
1. 双击运行 face_detection.exe 或 rtsp_face_recognition.exe
2. 或者在命令行中运行：
   rtsp_face_recognition.exe rtsp://admin:password@192.168.1.100:554/stream

快捷键：
  Q - 退出程序
  S - 截图保存
  R - 注册新人脸
  L - 列出已注册人脸
  D - 删除人脸
  H - 显示帮助

注意：
- 首次运行会自动下载模型文件（需要网络连接）
- 截图保存在 %APPDATA%\rtsp-face-recognition\captures\ 目录
- 人脸数据库保存在 %APPDATA%\rtsp-face-recognition\face_database.json

常见问题：
1. 缺少 DLL 错误 - 需要安装 Visual C++ Redistributable
2. 无法连接 RTSP - 检查防火墙和摄像头地址
3. 程序崩溃 - 确保有 OpenCV 运行时库

技术支持：
- OpenCV: https://opencv.org/
- ONNX Runtime: https://onnxruntime.ai/
EOF

echo ""
echo "==================================="
echo -e "${GREEN}构建完成！${NC}"
echo "==================================="
echo ""
echo "输出文件："
ls -lh dist/windows/
echo ""
echo "分发包位置：dist/windows/"
echo ""
echo "注意：运行程序需要以下依赖："
echo "  1. OpenCV Windows DLLs (opencv_*.dll)"
echo "  2. ONNX Runtime DLLs (onnxruntime.dll)"
echo "  3. VC++ 运行时或 MinGW 运行时"
echo ""
echo "建议：使用依赖收集工具收集所有 DLL："
echo "  python -m pip install pywin32"
echo "  python -c \"import subprocess; subprocess.run(['python', '-m', 'PyInstaller', '--onedir', 'rtsp_face_recognition.exe'])\""
