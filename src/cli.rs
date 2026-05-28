use clap::Parser;

/// grep_redo — 一个对标 grep 的文本搜索工具
#[derive(Debug, Parser, Clone)]
#[command(name = "grep_redo", version = "0.1.0", about = "Search text with patterns")]
pub struct Cli {
    // 目标字符串，必填参数
    #[arg(required = true, value_name = "PATTERN")]
    pub target: String,

    // ========== 输入源控制 ==========
    #[arg(value_name = "FILES")]
    pub filenames: Vec<String>,
    // ========== 匹配控制 ==========
    /// 忽略大小写 (was -u/--upper)
    #[arg(short = 'i', long = "ignore-case")]
    pub ignore_case: bool,

    /// 反向匹配，选中不匹配的行
    #[arg(short = 'v', long = "invert-match")]
    pub invert_match: bool,

    /// 将模式视为扩展正则表达式（默认启用）
    #[arg(short = 'E', long = "extended-regexp")]
    pub extended_regexp: bool,

    /// 将模式视为固定字符串（禁用正则）
    #[arg(short = 'F', long = "fixed-strings", conflicts_with = "extended_regexp")]
    pub fixed_strings: bool,

    /// 仅匹配整个单词
    #[arg(short = 'w', long = "word-regexp")]
    pub word_regexp: bool,

    /// 仅匹配整行
    #[arg(short = 'x', long = "line-regexp")]
    pub line_regexp: bool,

    /// 从文件中读取模式（每行一个）
    #[arg(short = 'f', long = "file")]
    pub file: Option<String>,

    /// 使用 PATTERN 作为搜索模式（可多次指定）
    #[arg(short = 'e', long = "regexp")]
    pub regexp: Option<String>,

    // ========== 输出控制 ==========
    /// 显示行号 (was -l/--line)
    #[arg(short = 'n', long = "line-number")]
    pub line_number: bool,

    /// 显示所有行，高亮匹配项 (was --all)
    #[arg(long = "all")]
    pub all: bool,

    /// 仅输出匹配的文本片段
    #[arg(short = 'o', long = "only-matching")]
    pub only_matching: bool,

    /// 仅输出包含匹配的文件名
    #[arg(short = 'l', long = "files-with-matches")]
    pub files_with_matches: bool,

    /// 仅输出不包含匹配的文件名
    #[arg(short = 'L', long = "files-without-match")]
    pub files_without_match: bool,

    /// 计数匹配行数
    #[arg(short = 'c', long = "count")]
    pub count: bool,

    /// 静默模式，仅通过退出码表示结果
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// 显示文件名前缀
    #[arg(short = 'H', long = "with-filename")]
    pub with_filename: bool,

    /// 抑制文件名前缀
    #[arg(long = "no-filename")]
    pub no_filename: bool,

    // ========== 上下文控制 ==========
    /// 匹配后显示 NUM 行
    #[arg(short = 'A', long = "after-context")]
    pub after_context: Option<usize>,

    /// 匹配前显示 NUM 行
    #[arg(short = 'B', long = "before-context")]
    pub before_context: Option<usize>,

    /// 匹配前后各显示 NUM 行
    #[arg(short = 'C', long = "context")]
    pub context: Option<usize>,

    // ========== 文件/目录控制 ==========
    /// 递归搜索子目录
    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,

    /// 搜索时包含匹配的文件名模式
    #[arg(long = "include")]
    pub include: Option<String>,

    /// 搜索时排除匹配的文件名模式
    #[arg(long = "exclude")]
    pub exclude: Option<String>,

    /// 排除匹配的目录
    #[arg(long = "exclude-dir")]
    pub exclude_dir: Option<String>,

    /// 读取到 NUM 个匹配后停止
    #[arg(short = 'm', long = "max-count")]
    pub max_count: Option<usize>,

    // ========== 其他 ==========
    /// 将二进制文件视为文本文件
    #[arg(short = 'a', long = "text", conflicts_with = "all")]
    pub text: bool,

    /// 并行线程数（0 = 自动使用所有 CPU 核心）
    #[arg(short = 'j', long = "threads", default_value = "0")]
    pub threads: usize,

    // ========== 编码控制 ==========
    /// 输入文件编码（auto=自动检测BOM，其他: utf-8, utf-16le, gbk, big5...）
    #[arg(long = "input-encoding", default_value = "auto")]
    pub input_encoding: String,

    /// 输出编码（默认 utf-8，其他: gbk, utf-16le...）
    #[arg(long = "output-encoding", default_value = "utf-8")]
    pub output_encoding: String,
}

