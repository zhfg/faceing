# Windows 编译指南

本文档介绍如何在 Windows 上编译 RTSP 人脸识别系统，包括原生编译和交叉编译两种方法。

## 目录

1. [方案一：GitHub Actions 自动构建（推荐）](#方案一github-actions-自动构建推荐)
2. [方案二：Windows 本地编译](#方案二windows-本地编译)
3. [方案三：Linux 交叉编译到 Windows](#方案三linux-交叉编译到-windows)
4. [方案四：Docker 交叉编译](#方案四docker-交叉编译)

---

## 方案一：GitHub Actions 自动构建（推荐）

最简单的方法是使用 GitHub Actions 自动构建 Windows 版本。

### 步骤

1. 将代码推送到 GitHub 仓库
2. GitHub Actions 会自动运行构建（见 `.github/workflows/build.yml`）
3. 在 Actions 页面下载构建好的 `.exe` 文件

### 优点

- 无需配置本地环境
- 自动处理依赖
- 生成的可执行文件包含所有必要的静态链接

---

## 方案二：Windows 本地编译

### 前提条件

- Windows 10/11
- Visual Studio 2019/2022（包含 C++ 工具链）
- Rust 工具链
- vcpkg

### 步骤

#### 1. 安装 Visual Studio

下载并安装 [Visual Studio Community](https://visualstudio.microsoft.com/vs/community/)，确保勾选：
- 使用 C++ 的桌面开发
- Windows 10/11 SDK

#### 2. 安装 Rust

```powershell
# 使用 rustup 安装
winget install Rustlang.Rustup

# 重启终端后
rustup default stable
```

#### 3. 安装 vcpkg 和 OpenCV

```powershell
# 克隆 vcpkg
git clone https://github.com/Microsoft/vcpkg.git C:\vcpkg
cd C:\vcpkg
.\bootstrap-vcpkg.bat

# 安装 OpenCV (静态链接，MD 运行时)
.\vcpkg install opencv4[core,dnn,ffmpeg]:x64-windows-static-md

# 集成到 Visual Studio
.\vcpkg integrate install
```

#### 4. 设置环境变量

```powershell
[System.Environment]::SetEnvironmentVariable("VCPKG_ROOT", "C:\vcpkg", "User")
[System.Environment]::SetEnvironmentVariable("VCPKG_DEFAULT_TRIPLET", "x64-windows-static-md", "User")

# 重新加载环境变量
$env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
```

#### 5. 编译项目

```powershell
# 克隆项目
git clone <repository-url>
cd rtsp-face-recognition

# 编译
cargo build --release

# 生成的可执行文件在 target/release/
ls target/release/*.exe
```

### 运行依赖

生成的 `.exe` 文件需要以下运行时文件：

1. **OpenCV DLLs**（如果使用动态链接）：
   - `opencv_world455.dll`
   - `opencv_videoio_ffmpeg455_64.dll`

2. **VC++ 运行时**（如果使用动态链接）：
   - `vcruntime140.dll`
   - `msvcp140.dll`

如果使用静态链接（`-C target-feature=+crt-static`），则不需要这些文件。

---

## 方案三：Linux 交叉编译到 Windows

### 前提条件

- Linux 系统（Ubuntu/Debian）
- MinGW-w64 交叉编译器
- Rust 工具链

### 步骤

#### 1. 安装依赖

```bash
sudo apt-get update
sudo apt-get install -y \
    mingw-w64 \
    g++-mingw-w64-x86-64 \
    cmake \
    git \
    wget \
    unzip
```

#### 2. 安装 Rust 和 Windows 目标

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 添加 Windows 目标
rustup target add x86_64-pc-windows-gnu
```

#### 3. 下载预编译的 OpenCV Windows 库

```bash
# 下载预编译的 OpenCV MinGW 版本
wget https://github.com/huihut/OpenCV-MinGW-Build/archive/refs/tags/OpenCV-4.5.5-x64.zip -O opencv.zip
unzip opencv.zip
sudo mv OpenCV-MinGW-Build-OpenCV-4.5.5-x64 /opt/opencv-win
```

#### 4. 设置环境变量

```bash
# 添加到 ~/.bashrc 或 ~/.zshrc
export OPENCV_LINK_LIBS="opencv_world455,opencv_videoio_ffmpeg455_64"
export OPENCV_LINK_PATHS="/opt/opencv-win/x64/mingw/lib"
export OPENCV_INCLUDE_PATHS="/opt/opencv-win/include"
```

#### 5. 编译

```bash
# 使用项目提供的脚本
chmod +x build-windows.sh
./build-windows.sh

# 或手动编译
cargo build --release --target x86_64-pc-windows-gnu
```

输出文件在 `target/x86_64-pc-windows-gnu/release/` 目录。

---

## 方案四：Docker 交叉编译

### 步骤

#### 1. 构建 Docker 镜像

```bash
docker build -f Dockerfile.windows -t rtsp-face-win .
```

#### 2. 运行编译

```bash
docker run -v $(pwd):/workspace rtsp-face-win
```

#### 3. 获取输出

编译完成后，Windows 可执行文件在 `dist/windows/` 目录。

---

## 打包和分发

### 收集依赖 DLL

编译完成后，如果使用了动态链接，需要收集依赖的 DLL 文件。

#### 方法 1：使用 Dependency Walker

下载 [Dependencies](https://github.com/lucasg/Dependencies) 工具，分析可执行文件的依赖。

#### 方法 2：使用 Python 脚本

```python
# collect_deps.py
import subprocess
import shutil
import os

def collect_deps(exe_path, output_dir):
    """收集可执行文件的依赖"""
    # 使用 dumpbin 或 objdump 分析依赖
    result = subprocess.run(['ldd', exe_path], capture_output=True, text=True)

    for line in result.stdout.split('\n'):
        if '=>' in line and 'windows' in line.lower():
            parts = line.split('=>')
            if len(parts) == 2:
                dll_path = parts[1].strip().split()[0]
                if os.path.exists(dll_path):
                    shutil.copy2(dll_path, output_dir)
                    print(f"Copied: {dll_path}")

if __name__ == '__main__':
    collect_deps('target/release/rtsp_face_recognition.exe', 'dist/windows')
```

### 创建安装包

#### 使用 Inno Setup

创建 `setup.iss` 文件：

```pascal
[Setup]
AppName=RTSP Face Recognition
AppVersion=1.0
DefaultDirName={pf}\RTSPFaceRecognition
DefaultGroupName=RTSP Face Recognition
OutputDir=dist
OutputBaseFilename=rtsp-face-recognition-setup

[Files]
Source: "dist\windows\rtsp_face_recognition.exe"; DestDir: "{app}"
Source: "dist\windows\face_detection.exe"; DestDir: "{app}"
Source: "dist\windows\*.dll"; DestDir: "{app}"

[Icons]
Name: "{group}\RTSP Face Recognition"; Filename: "{app}\rtsp_face_recognition.exe"
```

编译：
```bash
"C:\Program Files (x86)\Inno Setup 6\ISCC.exe" setup.iss
```

#### 使用 7-Zip 自解压

```bash
# 创建压缩包
7z a -sfx7z.sfx dist/rtsp-face-recognition.exe dist/windows/*
```

---

## 常见问题

### 1. 编译错误：找不到 OpenCV

**错误信息**：
```
error: could not find native static library `opencv_core`
```

**解决方案**：
- 确保设置了正确的 `OPENCV_LINK_PATHS` 和 `OPENCV_INCLUDE_PATHS`
- 检查 OpenCV 库文件名是否正确（可能包含版本号，如 `opencv_core455`）

### 2. 运行时错误：缺少 DLL

**错误信息**：
```
The code execution cannot proceed because opencv_world455.dll was not found
```

**解决方案**：
- 将所有依赖的 DLL 放在可执行文件同一目录
- 或使用静态链接编译

### 3. 链接错误：未定义的符号

**错误信息**：
```
undefined reference to `cv::VideoCapture::VideoCapture()`
```

**解决方案**：
- 确保链接了所有需要的 OpenCV 模块
- 检查 MinGW 和 MSVC 库不能混用

### 4. GitHub Actions 构建失败

**解决方案**：
- 检查 `vcpkg.json` 或 `vcpkg` 命令中的包名是否正确
- 查看 Actions 日志获取详细信息

---

## 参考链接

- [OpenCV 官方文档](https://docs.opencv.org/)
- [Rust OpenCV 绑定](https://github.com/twistedfall/opencv-rust)
- [vcpkg 文档](https://vcpkg.io/en/docs/)
- [MinGW-w64 项目](https://www.mingw-w64.org/)
