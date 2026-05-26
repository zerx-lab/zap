#!/usr/bin/env bash
# Zap remote-server 二进制的预安装检查。
#
# stdout 输出结构化 key=value 摘要。退出码 0 表示探测完成;
# 非 0 表示探测过程失败,客户端会按 `status=unknown` 处理并 fail open。
#
# 重要:Zap Linux remote-server 现在由 zap_release.yml 以
# `x86_64-unknown-linux-musl` 目标静态链接构建(static-musl)。产物不依赖
# 宿主的动态 libc,因此可以在任意 Linux x86_64 主机上运行 —— 包括旧 glibc
# 发行版(CentOS 7 = 2.17、Amazon Linux 2 = 2.26、Ubuntu 20.04 / Debian 11
# = 2.31)以及 musl 发行版(Alpine 等)。
#
# 既然二进制是静态的,libc 探测不再用于「门禁」,只作为遥测信息保留。

set -u

# 历史字段:保留 required_glibc 以兼容旧客户端的解析逻辑。
# 静态 musl 二进制实际上没有 glibc 下限,此处仅为向后兼容输出,
# 不再参与下面的 status 判定。
required_glibc="2.17"
echo "required_glibc=${required_glibc}"

# 1. 识别 libc family,并在 glibc 场景下识别版本(纯遥测,不影响 status)。
libc_family="unknown"
libc_version=""

if version=$(getconf GNU_LIBC_VERSION 2>/dev/null); then
    # 输出形如: "glibc 2.35"
    libc_family="glibc"
    libc_version="${version##* }"
elif ldd_out=$(ldd --version 2>&1 | head -n1); then
    case "$ldd_out" in
        *musl*)   libc_family="musl"   ;;
        *uClibc*) libc_family="uclibc" ;;
        *)
            v=$(printf '%s\n' "$ldd_out" | grep -oE '[0-9]+\.[0-9]+' | head -n1)
            if [ -n "$v" ]; then
                libc_family="glibc"
                libc_version="$v"
            fi
            ;;
    esac
fi

echo "libc_family=${libc_family}"
[ -n "$libc_version" ] && echo "libc_version=${libc_version}"

# 2. 判断支持状态。
#
# remote-server 是静态 musl 二进制,不链接宿主 libc,所以任何 glibc 版本
# (含 2.35 以下)以及 musl / uclibc 宿主都能运行。只要成功识别出这是一台
# Linux x86_64 主机,就报告 `supported`;探测不出任何 libc 线索(连
# getconf 和 ldd 都没有)时回退 `unknown`,让客户端 fail open 照常尝试安装。
status="unknown"
reason=""

if [ "$libc_family" = "glibc" ] \
   || [ "$libc_family" = "musl" ] \
   || [ "$libc_family" = "uclibc" ] \
   || [ "$libc_family" = "bionic" ]; then
    status="supported"
fi

echo "status=${status}"
if [ -n "$reason" ]; then
    echo "reason=${reason}"
fi
