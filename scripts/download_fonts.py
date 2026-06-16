#!/usr/bin/env python3
"""
下载 CmdHub 打包字体：
- Noto Sans SC（UI 界面，含中文）
- JetBrains Mono（终端输出）

用法:
  python3 scripts/download_fonts.py                         # 直连下载
  python3 scripts/download_fonts.py --proxy                 # 通过 127.0.0.1:7890
  python3 scripts/download_fonts.py --proxy=http://host:8888  # 自定义代理
"""

import argparse
import os
import shutil
import ssl
import sys
import urllib.request
import zipfile

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FONTS_DIR = os.path.join(PROJECT_ROOT, "fonts")

# 直连模式使用默认 SSL 上下文，代理模式下会替换为不验证证书的上下文
_ssl_context: ssl.SSLContext | None = None

NOTO_SANS_URL = (
    "https://github.com/googlefonts/noto-cjk/releases/download/Sans2.004/"
    "08_NotoSansCJKsc.zip"
)
JETBRAINS_MONO_URL = (
    "https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/"
    "JetBrainsMono-2.304.zip"
)


def setup_proxy(proxy: str | None) -> None:
    """配置全局代理（如果指定）"""
    global _ssl_context
    if proxy is None:
        return
    # 补全 scheme
    if not proxy.startswith("http://") and not proxy.startswith("https://"):
        proxy = f"http://{proxy}"
    print(f"使用代理: {proxy}")
    # 代理模式下 HTTPS 证书链常不完整，跳过验证
    _ssl_context = ssl._create_unverified_context()
    handler = urllib.request.ProxyHandler({
        "http": proxy,
        "https": proxy,
    })
    opener = urllib.request.build_opener(
        handler,
        urllib.request.HTTPSHandler(context=_ssl_context),
    )
    urllib.request.install_opener(opener)


def download_and_extract(
    url: str, extract_to: str, font_subdir: str, targets: list[str]
) -> list[str]:
    """下载 zip 并提取指定字体文件，返回提取到的文件路径列表"""
    os.makedirs(extract_to, exist_ok=True)
    zip_path = os.path.join(extract_to, "temp_fonts.zip")

    print(f"\n下载 {url}...")
    urllib.request.urlretrieve(url, zip_path)

    print("解压...")
    with zipfile.ZipFile(zip_path, "r") as zf:
        zf.extractall(extract_to)

    os.remove(zip_path)

    # 移动目标文件到 fonts/
    result = []
    for target in targets:
        src = os.path.join(extract_to, font_subdir, target)
        dst = os.path.join(FONTS_DIR, target)
        if os.path.exists(src):
            shutil.copy2(src, dst)
            result.append(dst)
            print(f"  → {target}")
        else:
            print(f"  ⚠ 未找到 {target}，跳过")

    # 清理解压目录
    shutil.rmtree(extract_to, ignore_errors=True)
    return result


def main():
    parser = argparse.ArgumentParser(description="下载 CmdHub 打包字体")
    parser.add_argument(
        "--proxy",
        nargs="?",
        const="127.0.0.1:7890",
        default=None,
        metavar="HOST:PORT",
        help="通过代理下载，默认 127.0.0.1:7890（例: --proxy 或 --proxy=10.0.0.1:8888）",
    )
    args = parser.parse_args()

    setup_proxy(args.proxy)

    os.makedirs(FONTS_DIR, exist_ok=True)

    download_and_extract(
        NOTO_SANS_URL,
        os.path.join(PROJECT_ROOT, ".font_extract_noto"),
        "NotoSansSC",
        ["NotoSansSC-Regular.otf", "NotoSansSC-Bold.otf"],
    )

    download_and_extract(
        JETBRAINS_MONO_URL,
        os.path.join(PROJECT_ROOT, ".font_extract_jb"),
        "fonts/ttf",
        ["JetBrainsMono-Regular.ttf", "JetBrainsMono-Bold.ttf"],
    )

    print(f"\n字体已下载到 {FONTS_DIR}")
    print("然后在 pubspec.yaml 中确认 flutter → fonts 声明已启用。")


if __name__ == "__main__":
    main()
