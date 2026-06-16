#!/usr/bin/env python3
"""
CmdHub 图标部署脚本
从 refs/app-icons/ 读取图标，分发到各平台目录。
用法: python3 scripts/deploy_icons.py
"""

import os
import shutil
import struct
import subprocess
import sys

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICONS_DIR = os.path.join(PROJECT_ROOT, "refs", "app-icons")

# 忽略的文件（不视为图标资源）
IGNORED = {
    "terminal_256dp_FFFFFF_FILL0_wght400_GRAD0_opsz48.svg",
    ".DS_Store",
    "icon.svg",  # 源 SVG，各平台已导出具体尺寸 PNG
}


def sips_resize(src: str, dst: str, size: int) -> None:
    """调用 macOS sips 缩放图片到指定尺寸"""
    subprocess.run(
        ["sips", "-z", str(size), str(size), src, "--out", dst],
        check=True, capture_output=True,
    )


def copy_file(src: str, dst: str) -> None:
    """复制文件，自动创建目标目录"""
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    shutil.copy2(src, dst)


def deploy_macos() -> None:
    """部署 macOS 应用图标到 Assets.xcassets"""
    src_dir = os.path.join(ICONS_DIR, "macOS")
    if not os.path.isdir(src_dir):
        print("[macOS] 未找到 macOS 图标目录，跳过")
        return

    dst_dir = os.path.join(
        PROJECT_ROOT, "macos", "Runner", "Assets.xcassets", "AppIcon.appiconset"
    )
    os.makedirs(dst_dir, exist_ok=True)

    sizes = [16, 32, 64, 128, 256, 512, 1024]
    for s in sizes:
        src = os.path.join(src_dir, f"{s}.png")
        dst = os.path.join(dst_dir, f"app_icon_{s}.png")
        if not os.path.exists(src):
            print(f"[macOS] 警告: 缺少 {s}px 图标 ({src})")
            continue
        copy_file(src, dst)
        print(f"[macOS] app_icon_{s}.png")

    # 刷新 Contents.json 时间戳，确保 Xcode 重新编译 Asset Catalog
    contents_json = os.path.join(dst_dir, "Contents.json")
    if os.path.exists(contents_json):
        os.utime(contents_json, None)
        print("[macOS] Contents.json 时间戳已刷新")


def deploy_windows() -> None:
    """生成多尺寸 .ico 并部署到 Windows runner"""
    src_dir = os.path.join(ICONS_DIR, "macOS")
    if not os.path.isdir(src_dir):
        print("[Windows] 未找到 macOS 图标目录用作来源，跳过")
        return

    sizes = [16, 32, 48, 64, 256]
    png_data = {}

    # 48px 可能需要从大图生成
    prebuilt_48 = os.path.join(ICONS_DIR, "macOS", "48.png")
    if not os.path.exists(prebuilt_48):
        # 从 128px 缩放生成
        src_128 = os.path.join(ICONS_DIR, "macOS", "128.png")
        if os.path.exists(src_128):
            tmp = os.path.join(PROJECT_ROOT, ".deploy_icons_48.png")
            sips_resize(src_128, tmp, 48)
            with open(tmp, "rb") as f:
                png_data[48] = f.read()
            os.remove(tmp)
        else:
            print("[Windows] 警告: 无法生成 48px（缺少 128px 来源）")

    for s in sizes:
        if s == 48 and png_data.get(s):
            continue
        path = os.path.join(ICONS_DIR, "macOS", f"{s}.png")
        if os.path.exists(path):
            with open(path, "rb") as f:
                png_data[s] = f.read()
        else:
            print(f"[Windows] 警告: 缺少 {s}px 来源")

    if not png_data:
        print("[Windows] 无可用图标数据，跳过")
        return

    # 构建 .ico 文件
    ico_dir = bytearray()
    ico_data = bytearray()
    offset = 6 + 16 * len(png_data)

    for s in sizes:
        data = png_data.get(s)
        if data is None:
            continue
        w = 0 if s == 256 else s
        h = 0 if s == 256 else s
        ico_dir += struct.pack("<BBBBHHII", w, h, 0, 0, 1, 32, len(data), offset)
        ico_data += data
        offset += len(data)

    header = struct.pack("<HHH", 0, 1, len(png_data))
    dst = os.path.join(PROJECT_ROOT, "windows", "runner", "resources", "app_icon.ico")
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    with open(dst, "wb") as f:
        f.write(header + ico_dir + ico_data)
    print(f"[Windows] app_icon.ico ({len(png_data)} sizes)")


def deploy_android() -> None:
    """部署 Android 图标到 mipmap 各密度目录"""
    src_dir = os.path.join(ICONS_DIR, "Android")
    if not os.path.isdir(src_dir):
        print("[Android] 未找到 Android 图标目录，跳过")
        return

    mapping = {
        "mdpi": "mdpi.png",
        "hdpi": "hdpi.png",
        "xhdpi": "xhdpi.png",
        "xxhdpi": "xxhdpi.png",
        "xxxhdpi": "xxxhdpi.png",
    }

    for density, filename in mapping.items():
        src = os.path.join(src_dir, filename)
        dst = os.path.join(
            PROJECT_ROOT, "android", "app", "src", "main", "res",
            f"mipmap-{density}", "ic_launcher.png",
        )
        if not os.path.exists(src):
            print(f"[Android] 警告: 缺少 {density} ({src})")
            continue
        copy_file(src, dst)
        print(f"[Android] mipmap-{density}/ic_launcher.png")


def deploy_linux() -> None:
    """部署 Linux 图标到 linux/icons/"""
    src_dir = os.path.join(ICONS_DIR, "macOS")
    if not os.path.isdir(src_dir):
        print("[Linux] 未找到 macOS 图标目录用作来源，跳过")
        return

    dst_dir = os.path.join(PROJECT_ROOT, "linux", "icons")
    os.makedirs(dst_dir, exist_ok=True)

    sizes = [16, 32, 64, 128, 256, 512]

    for s in sizes:
        src = os.path.join(src_dir, f"{s}.png")
        dst = os.path.join(dst_dir, f"{s}.png")
        if not os.path.exists(src):
            print(f"[Linux] 警告: 缺少 {s}px ({src})")
            continue
        copy_file(src, dst)

    # 48px 需要从大图生成
    src_48 = os.path.join(src_dir, "48.png")
    dst_48 = os.path.join(dst_dir, "48.png")
    if os.path.exists(src_48):
        copy_file(src_48, dst_48)
    else:
        # 从 128px 缩放
        src_128 = os.path.join(src_dir, "128.png")
        if os.path.exists(src_128):
            tmp = os.path.join(PROJECT_ROOT, ".deploy_linux_48.png")
            sips_resize(src_128, tmp, 48)
            copy_file(tmp, dst_48)
            os.remove(tmp)
        else:
            print("[Linux] 警告: 无法生成 48px")

    # 符号链接
    symlink_src = os.path.join(dst_dir, "512.png")
    symlink_dst = os.path.join(dst_dir, "app.png")
    if os.path.exists(symlink_dst):
        os.remove(symlink_dst)
    os.symlink(os.path.relpath(symlink_src, os.path.dirname(symlink_dst)), symlink_dst)

    print(f"[Linux] icons/ 目录更新完成")


def main():
    if not os.path.isdir(ICONS_DIR):
        print(f"错误: 未找到图标目录 {ICONS_DIR}")
        print("请确认 refs/app-icons/ 存在且包含图标文件。")
        sys.exit(1)

    # 列出源目录中可识别的图标文件（用于提示）
    source_files = []
    for root, dirs, files in os.walk(ICONS_DIR):
        for f in files:
            if f in IGNORED or f == ".DS_Store":
                continue
            rel = os.path.relpath(os.path.join(root, f), ICONS_DIR)
            source_files.append(rel)

    print(f"图标源: {ICONS_DIR}")
    print(f"发现 {len(source_files)} 个图标文件")
    print("=" * 50)

    deploy_macos()
    deploy_windows()
    deploy_android()
    deploy_linux()

    print("=" * 50)
    print("完成。运行 flutter clean && flutter run -d macos 刷新缓存。")


if __name__ == "__main__":
    main()
