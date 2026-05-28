# grep_redo

一个用 Rust 实现的对标 `grep` 的文本搜索工具，支持多编码、正则、递归搜索、上下文、流式读取、多线程并行。

## 功能一览

| 功能 | 选项 | 状态 |
|------|------|------|
| 固定字符串搜索 | 默认 | ✅ |
| 忽略大小写 | `-i` | ✅ |
| 正则表达式 | `-E` | ✅ |
| 行号显示 | `-n` | ✅ |
| 反向匹配 | `-v` | ✅ |
| 全文高亮 | `--all` | ✅ |
| 上下文行 | `-A` / `-B` / `-C` | ✅ |
| 计数 | `-c` | ✅ |
| 静默模式 | `-q` | ✅ |
| 仅文件名 | `-l` / `-L` | ✅ |
| 整词匹配 | `-w` | ✅ |
| 整行匹配 | `-x` | ✅ |
| 递归搜索 | `-r` | ✅ |
| 文件过滤 | `--include` / `--exclude` / `--exclude-dir` | ✅ |
| 通配符 | `*` `?` `[]` | ✅ |
| 多文件 | `grep_redo pattern file1 file2` | ✅ |
| 多编码输入 | `--input-encoding` (UTF-8/16, GBK, Big5…) | ✅ |
| 多编码输出 | `--output-encoding` | ✅ |
| 自动 BOM 检测 | `--input-encoding auto` | ✅ |
| 多线程 | `-j` (默认自动) | ✅ |
| 流式读取 | 逐行处理，不加载全部到内存 | ✅ |
| 二进制静默跳过 | 非文本文件自动忽略 | ✅ |
| stdin 输入 | 无文件参数时从标准输入读取 | ✅ |

## 安装

```bash
git clone https://github.com/as061125/my_grep
cd my_grep
cargo build --release
```

编译产物在 `./target/release/grep_redo.exe`（Windows）或 `./target/release/grep_redo`（Linux/macOS）。

### 添加到 PATH

编译后只需做一次，之后可以直接在终端敲 `grep_redo`。

#### Linux / macOS

```bash
# 方法一：复制到系统路径（推荐）
sudo cp ./target/release/grep_redo /usr/local/bin/

# 方法二：添加到用户 PATH（编辑 ~/.bashrc 或 ~/.zshrc）
echo 'export PATH="$PATH:'$(pwd)'/target/release"' >> ~/.bashrc
source ~/.bashrc
```

#### Windows

```powershell
# 方法一：复制到系统路径（管理员 PowerShell）
Copy-Item .\target\release\grep_redo.exe C:\Windows\System32\

# 方法二：添加到用户 PATH
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$newPath = "$userPath;$((Get-Item .\target\release\).FullName)"
[Environment]::SetEnvironmentVariable("Path", $newPath, "User")
# 重启终端后生效
```

添加完成后验证：

```bash
grep_redo --version   # 应输出版本号
grep_redo --help      # 应显示帮助信息
```

## 使用示例

### 基本搜索

```bash
# 精确匹配
grep_redo "hello" file.txt

# 忽略大小写
grep_redo -i "hello" file.txt

# 显示行号
grep_redo -n "hello" file.txt

# 正则表达式
grep_redo -E "h[a-z]+o" file.txt

# 反向匹配（显示不包含的行）
grep_redo -v "hello" file.txt
```

### 多文件和递归

```bash
# 搜索多个文件
grep_redo "hello" file1.txt file2.txt

# 通配符
grep_redo "hello" *.txt

# 递归搜索目录
grep_redo -r "hello" .

# 递归 + 文件类型过滤
grep_redo -r "hello" --include "*.rs" .

# 递归 + 排除目录
grep_redo -r "hello" --exclude-dir target .
```

### 上下文

```bash
# 前后各 2 行
grep_redo -C 2 "error" log.txt

# 匹配后 3 行
grep_redo -A 3 "error" log.txt

# 匹配前 1 行
grep_redo -B 1 "error" log.txt
```

### 编码支持

```bash
# 自动检测 BOM（UTF-8 / UTF-16）
grep_redo "关键词" file.txt

# 指定 GBK 输入
grep_redo "关键词" file.txt --input-encoding gbk

# 输出为 GBK（解决终端乱码）
grep_redo "关键词" file.txt --input-encoding gbk --output-encoding gbk
```

### 其他

```bash
# 仅显示匹配的文件名
grep_redo -l "hello" *.txt

# 计数
grep_redo -c "hello" file.txt

# 静默模式（仅通过退出码判断）
grep_redo -q "hello" file.txt && echo "有匹配"

# 指定线程数
grep_redo -j 4 -r "hello" .

# 全文高亮
grep_redo --all "hello" file.txt

# 管道输入
cat file.txt | grep_redo "hello"
```

## 架构

```
src/
├── main.rs          # 入口：Cli::parse() → engine::run()
├── lib.rs           # 模块注册
├── cli.rs           # Cli 结构体（clap 定义，31 个参数）
├── matcher.rs       # 文件读取 + 搜索匹配 + 流式/并行
│   ├── get_content()      # 全量读取（用于 -v / --all / 上下文）
│   └── search_stream()    # 流式搜索（逐行匹配回调）
├── engine.rs        # 调度引擎：策略模式输出
│   ├── run_memory()       # 全量路径（-v / --all / -C）
│   └── run_streaming()    # 流式路径（默认模式）
└── encoding.rs      # 编码解码工具（UTF-8/16, GBK, Big5…）
```

### 数据流

```
CLI args
  │
  ▼
Cli::parse()  ──→  engine::run()
                      │
            ┌─────────┴─────────┐
            ▼                   ▼
      run_streaming()      run_memory()
            │                   │
            ▼                   ▼
    search_stream()        get_content()
    (BufReader逐行)        + search()
            │              (全文内存)
            ▼                   │
     逐行调用 on_match    Vec<(file,line,text)>
            │                   │
            ▼                   ▼
      write_line()        OutputStrategy
      (编码输出)          (策略输出)
```

## 测试

```bash
python test_robustness.py
```

运行 52 项鲁棒性测试，覆盖基础匹配、编码兼容、边界情况、组合模式等。

## 依赖

- [clap](https://crates.io/crates/clap) — 命令行参数解析
- [regex](https://crates.io/crates/regex) — 正则表达式
- [encoding_rs](https://crates.io/crates/encoding_rs) — 多编码支持
- [glob](https://crates.io/crates/glob) — 通配符展开
- [walkdir](https://crates.io/crates/walkdir) — 目录递归遍历
- [rayon](https://crates.io/crates/rayon) — 并行处理
