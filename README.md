# RTSP 流媒体人脸识别系统 (Rust 版)

基于 Rust + OpenCV 的高性能实时人脸识别系统，支持 RTSP 网络摄像头接入。

## 功能特点

- 纯 Rust 实现，高性能、内存安全
- 基于 OpenCV DNN 深度学习的人脸检测
- 人脸数据库管理（注册/删除/识别）
- 异步视频流处理
- 本地数据持久化
- 截图保存功能

## 系统要求

- Rust 1.75+ 
- OpenCV 4.x
- CMake 3.10+
- FFmpeg（用于 RTSP 解码）

## 安装依赖

### Ubuntu/Debian

```bash
# 安装系统依赖
sudo apt-get update
sudo apt-get install -y \
    cmake \
    libopencv-dev \
    libclang-dev \
    libssl-dev \
    pkg-config \
    ffmpeg \
    libavformat-dev \
    libavcodec-dev \
    libswscale-dev
```

### macOS

```bash
brew install cmake opencv ffmpeg pkg-config
```

### Windows (原生编译)

1. 安装 [vcpkg](https://vcpkg.io/)
2. 安装 OpenCV:
   ```powershell
   vcpkg install opencv4[core,dnn,ffmpeg]:x64-windows-static
   ```
3. 安装 LLVM (用于 bindgen):
   ```powershell
   winget install LLVM.LLVM
   ```
4. 设置环境变量:
   ```powershell
   $env:VCPKG_ROOT = "C:\vcpkg"
   $env:OPENCV_LINK_LIBS = "opencv_core,opencv_imgproc,opencv_videoio,opencv_highgui,opencv_imgcodecs,opencv_dnn"
   $env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
   ```
5. 编译:
   ```powershell
   cargo build --release
   ```

### Windows (交叉编译 from Linux)

见下方的 [Windows 交叉编译](#windows-交叉编译) 章节。

## 编译项目

```bash
# 克隆/下载项目后
cd rtsp-face-recognition

# 编译
cargo build --release
```

## 使用方法

### 1. 下载模型文件（首次运行）

```bash
cargo run --bin download_models
```

这会下载 OpenCV DNN 人脸检测模型到本地数据目录。

### 2. 简化版 - 仅人脸检测

```bash
cargo run --bin face_detection rtsp://admin:password@192.168.1.100:554/stream
```

或运行时输入 URL：
```bash
cargo run --bin face_detection
```

### 3. 完整版 - 人脸识别系统

```bash
cargo run --bin rtsp_face_recognition rtsp://admin:password@192.168.1.100:554/stream
```

## 操作快捷键

| 按键 | 功能 |
|------|------|
| `Q` | 退出程序 |
| `S` | 截图保存 |
| `R` | 注册新人脸（输入姓名） |
| `L` | 列出已注册人脸 |
| `D` | 删除人脸 |
| `H` | 显示帮助 |

## RTSP URL 格式

| 品牌 | URL 格式 |
|------|---------|
| 海康威视 | `rtsp://admin:密码@IP:554/Streaming/Channels/101` |
| 大华 | `rtsp://admin:密码@IP:554/cam/realmonitor?channel=1&subtype=0` |
| 宇视 | `rtsp://admin:密码@IP:554/video1` |
| 通用 | `rtsp://用户名:密码@IP:端口/stream` |

## 项目结构

```
.
├── Cargo.toml                  # Rust 项目配置
├── src/
│   ├── main.rs                 # 主程序入口
│   ├── lib.rs                  # 库模块
│   ├── capture.rs              # RTSP 视频捕获
│   ├── detector.rs             # 人脸检测（DNN）
│   ├── database.rs             # 人脸数据库
│   └── bin/
│       ├── face_detection.rs   # 简化版检测程序
│       └── download_models.rs  # 模型下载工具
├── captures/                   # 截图保存目录（自动生成）
└── README.md                   # 本文件
```

## 配置文件

数据文件存储在用户数据目录：

- **Linux**: `~/.local/share/rtsp-face-recognition/`
- **macOS**: `~/Library/Application Support/rtsp-face-recognition/`
- **Windows**: `%APPDATA%\rtsp-face-recognition\`

包含文件：
- `face_database.json` - 人脸数据库
- `models/` - 深度学习模型文件
- `captures/` - 截图保存目录

## 常见问题

### 编译错误：找不到 OpenCV

```bash
# 设置 OpenCV 环境变量
export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig:$PKG_CONFIG_PATH

# 或指定 OpenCV 路径
export OPENCV_INCLUDE_DIRS=/usr/include/opencv4
export OPENCV_LINK_LIBS=opencv_core,opencv_videoio,...
```

### 运行时错误：无法连接到 RTSP

1. 检查 URL 格式是否正确
2. 确认摄像头网络可达：`ping <camera_ip>`
3. 检查防火墙设置，确保 TCP 554 端口开放
4. 尝试用 VLC 播放器测试 URL

### 模型下载失败

手动下载模型文件：
1. 模型: https://github.com/opencv/opencv_3rdparty/raw/dnn_samples_face_detector_20170830/res10_300x300_ssd_iter_140000.caffemodel
2. 配置: https://raw.githubusercontent.com/opencv/opencv/master/samples/dnn/face_detector/deploy.prototxt

保存到数据目录的 `models/` 子目录下。

## Windows 编译

完整的 Windows 编译指南见 [docs/WINDOWS_BUILD.md](docs/WINDOWS_BUILD.md)。

### 快速开始（Docker 交叉编译）

```bash
# 使用 Docker 快速构建 Windows 版本
docker build -f Dockerfile.windows -t rtsp-face-win .
docker run -v $(pwd):/workspace rtsp-face-win
```

### 快速开始（GitHub Actions）

1. Fork 本仓库到 GitHub
2. 在 Actions 页面启用 workflows
3. 推送代码后自动构建 Windows 版本
4. 在 Actions 页面下载 `rtsp-face-recognition-windows` 工件

### 手动交叉编译

详细步骤见 [docs/WINDOWS_BUILD.md](docs/WINDOWS_BUILD.md)，包括：
- Windows 本地编译
- Linux 交叉编译
- Docker 交叉编译
- 依赖收集和打包

## Linux/macOS 编译

```bash
# 安装依赖后运行
./build.sh
```

## Windows 交叉编译（详细）

### 方法一：使用 Docker（推荐）

```bash
# 构建 Docker 镜像
docker build -f Dockerfile.windows -t rtsp-face-win .

# 运行编译
docker run -v $(pwd):/workspace rtsp-face-win
```

### 方法二：本地交叉编译

#### 1. 安装 MinGW 交叉编译器

```bash
# Ubuntu/Debian
sudo apt-get install mingw-w64 g++-mingw-w64-x86-64

# 添加 Rust 目标
rustup target add x86_64-pc-windows-gnu
```

#### 2. 下载 OpenCV Windows 库

```bash
# 下载预编译的 OpenCV Windows 库
wget https://github.com/opencv/opencv/releases/download/4.9.0/opencv-4.9.0-windows.exe

# 或使用 vcpkg
vcpkg install opencv4[core,dnn,ffmpeg]:x64-mingw-static
```

#### 3. 设置环境变量

```bash
export OPENCV_LINK_PATHS="/path/to/opencv/x64/mingw/lib"
export OPENCV_INCLUDE_PATHS="/path/to/opencv/include"
export OPENCV_LINK_LIBS="opencv_world490"
```

#### 4. 运行编译脚本

```bash
chmod +x build-windows.sh
./build-windows.sh
```

输出文件在 `dist/windows/` 目录。

### 方法三：使用 Cross 工具

```bash
# 安装 cross
cargo install cross

# 使用 cross 编译
cross build --release --target x86_64-pc-windows-gnu
```

### Windows 运行依赖

生成的 `.exe` 文件需要以下依赖才能运行：

1. **OpenCV DLLs**: `opencv_world490.dll`, `opencv_videoio_ffmpeg490_64.dll`
2. **ONNX Runtime**: `onnxruntime.dll`
3. **VC++ 运行时**: `vcruntime140.dll`, `msvcp140.dll`

可以使用依赖收集工具自动收集：

```powershell
# 使用 Dependency Walker 或类似工具
# 或使用 Python 的 cx_Freeze 打包
```

## 性能优化

- 使用发布模式编译：`cargo build --release`
- 降低输入分辨率以提高 FPS
- 调整 `recognition_interval` 参数控制识别频率

## 许可证

MIT License
