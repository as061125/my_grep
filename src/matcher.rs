use std::io::{self, BufRead, Read};
use std::fs;

use regex::Regex;
use rayon::prelude::*;

use crate::cli::Cli;
use crate::encoding;

// ============================================================
// 工具函数
// ============================================================

/// 读取文件并解码为 UTF-8
fn read_file_decoded(path: &str, input_encoding: &str) -> io::Result<String> {
    let bytes = fs::read(path)?;
    encoding::decode_bytes(&bytes, input_encoding)
}

/// 从 stdin 读取并解码为 UTF-8
fn read_stdin_decoded(input_encoding: &str) -> io::Result<String> {
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes)?;
    encoding::decode_bytes(&bytes, input_encoding)
}

/// 根据 --include / --exclude / --exclude-dir 过滤文件路径
fn should_include(path: &str, cli: &Cli) -> bool {
    if let Some(inc) = &cli.include {
        if let Ok(p) = glob::Pattern::new(inc) {
            if !p.matches(path) {
                return false;
            }
        }
    }
    if let Some(exc) = &cli.exclude {
        if let Ok(p) = glob::Pattern::new(exc) {
            if p.matches(path) {
                return false;
            }
        }
    }
    if let Some(exd) = &cli.exclude_dir {
        if path.contains(exd) {
            return false;
        }
    }
    true
}

/// 默认排除的目录
const IGNORED_DIRS: &[&str] = &["target", ".git", ".svn", ".hg", "node_modules", "__pycache__"];

/// 递归遍历目录
fn walk_dir_recursive(root: &str, cli: &Cli) -> io::Result<Vec<String>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if IGNORED_DIRS.contains(&name.as_ref()) {
                return false;
            }
            if let Some(exd) = &cli.exclude_dir {
                if name.as_ref() == exd.as_str() {
                    return false;
                }
            }
            true
        })
    {
        let entry = entry.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        if entry.file_type().is_file() {
            let path = entry.path().to_string_lossy().to_string();
            if should_include(&path, cli) {
                files.push(path);
            }
        }
    }
    Ok(files)
}

/// 处理单个文件参数（glob / 目录递归 / 普通文件），返回其所有可读来源
fn process_one_arg(file: &str, cli: &Cli) -> io::Result<Vec<(String, String)>> {
    let mut sources = Vec::new();

    if ['*', '[', '?'].iter().any(|&c| file.contains(c)) {
        let entries = glob::glob(file)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
        for entry in entries.flatten() {
            let path = entry.to_string_lossy().to_string();
            if should_include(&path, cli) {
                if let Ok(content) = read_file_decoded(&path, &cli.input_encoding) {
                    sources.push((path, content));
                }
            }
        }
    } else if cli.recursive && std::path::Path::new(file).is_dir() {
        let found = walk_dir_recursive(file, cli)?;
        for path in found {
            if let Ok(content) = read_file_decoded(&path, &cli.input_encoding) {
                sources.push((path, content));
            }
        }
    } else if should_include(file, cli) {
        let content = read_file_decoded(file, &cli.input_encoding)?;
        sources.push((file.to_string(), content));
    }

    Ok(sources)
}

/// 读取输入源的内容，返回 (来源名, UTF-8内容) 列表（文件读取并行化）
pub fn get_content(cli: &Cli) -> io::Result<Vec<(String, String)>> {
    if !cli.filenames.is_empty() {
        let nested: Vec<Vec<(String, String)>> = cli
            .filenames
            .par_iter()
            .map(|file| process_one_arg(file, cli))
            .collect::<io::Result<Vec<_>>>()?;
        Ok(nested.into_iter().flatten().collect())
    } else {
        let content = read_stdin_decoded(&cli.input_encoding)?;
        Ok(vec![("stdin".to_string(), content)])
    }
}

// ============================================================
// 匹配引擎 — 固定字符串 或 正则表达式
// ============================================================

/// 根据 CLI 参数编译正则（仅 -E 模式）
fn compile_regex(cli: &Cli) -> Result<Option<Regex>, String> {
    if !cli.extended_regexp {
        return Ok(None);
    }

    let mut pattern = cli.target.clone();
    if cli.word_regexp {
        pattern = format!(r"\b{}\b", &pattern);
    }
    if cli.line_regexp {
        pattern = format!(r"^{}$", &pattern);
    }

    regex::RegexBuilder::new(&pattern)
        .case_insensitive(cli.ignore_case)
        .build()
        .map(Some)
        .map_err(|e| format!("无效正则: {e}"))
}

/// 判断一行是否匹配
fn line_matches(line: &str, target: &str, ignore_case: bool, regex: Option<&Regex>) -> bool {
    if let Some(re) = regex {
        re.is_match(line)
    } else if ignore_case {
        line.to_lowercase().contains(&target.to_lowercase())
    } else {
        line.contains(target)
    }
}

/// 高亮一行中的匹配文本
fn highlight_line(line: &str, target: &str, ignore_case: bool, regex: Option<&Regex>) -> String {
    let spans: Vec<(usize, usize)> = if let Some(re) = regex {
        re.find_iter(line).map(|m| (m.start(), m.end())).collect()
    } else if ignore_case {
        let lower_line = line.to_lowercase();
        let lower_target = target.to_lowercase();
        lower_line
            .match_indices(&lower_target)
            .map(|(start, m)| (start, start + m.len()))
            .collect()
    } else {
        let mut s = Vec::new();
        let mut pos = 0;
        while let Some(start) = line[pos..].find(target) {
            let abs_start = pos + start;
            let end = abs_start + target.len();
            s.push((abs_start, end));
            pos = end;
        }
        s
    };

    if spans.is_empty() {
        return line.to_string();
    }

    let mut hl = line.to_string();
    for &(start, end) in spans.iter().rev() {
        hl.replace_range(start..end, &format!("\x1b[31m{}\x1b[0m", &line[start..end]));
    }
    hl
}

fn is_word_boundary(line: &str, pos: usize) -> bool {
    let prev = if pos == 0 { None } else { line[..pos].chars().last() };
    let next = line[pos..].chars().next();
    let is_word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    is_word(prev) != is_word(next)
}

// ============================================================
// 文件列表收集（供流式搜索用）
// ============================================================

/// 收集一个文件参数展开后的所有实际文件路径
fn collect_paths_from_arg(file: &str, cli: &Cli) -> io::Result<Vec<String>> {
    let mut paths = Vec::new();

    if ['*', '[', '?'].iter().any(|&c| file.contains(c)) {
        for entry in glob::glob(file)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?
            .flatten()
        {
            let path = entry.to_string_lossy().to_string();
            if should_include(&path, cli) {
                paths.push(path);
            }
        }
    } else if cli.recursive && std::path::Path::new(file).is_dir() {
        paths = walk_dir_recursive(file, cli)?;
    } else if should_include(file, cli) {
        paths.push(file.to_string());
    }

    Ok(paths)
}

/// 流式搜索，逐行匹配并调用 on_match(source_name, line_num, highlighted_text)
/// 返回匹配总数。仅用于非 -v / 非 --all 模式。
pub fn search_stream<F>(cli: &Cli, mut on_match: F) -> io::Result<usize>
where
    F: FnMut(&str, u32, &str),
{
    let regex = compile_regex(cli).unwrap_or_else(|e| {
        eprintln!("正则错误: {e}");
        std::process::exit(1);
    });
    let target = &cli.target;

    let mut total = 0;

    if !cli.filenames.is_empty() {
        for file in &cli.filenames {
            let paths = collect_paths_from_arg(file, cli)?;
            for path in &paths {
                total += stream_file(path, cli, target, regex.as_ref(), &mut on_match)?;
            }
        }
    } else {
        // stdin 流式读取
        let stdin = io::stdin().lock();
        for (i, result) in (0..).zip(stdin.lines()) {
            let line = result?;
            let line_num = i + 1;
            if emit_if_match("stdin", &line, line_num, target, regex.as_ref(), cli, &mut on_match) {
                total += 1;
            }
        }
    }

    Ok(total)
}

/// 流式处理单个文件
fn stream_file<F>(
    path: &str,
    cli: &Cli,
    target: &str,
    regex: Option<&Regex>,
    on_match: &mut F,
) -> io::Result<usize>
where
    F: FnMut(&str, u32, &str),
{
    let file = fs::File::open(path)?;
    let mut reader = io::BufReader::new(file);
    let mut total = 0usize;

    // 用户指定非 UTF-8 编码 → 全量读+解码；否则流式逐行读
    let is_utf8_enc = cli.input_encoding == "auto"
        || cli.input_encoding.eq_ignore_ascii_case("utf-8")
        || cli.input_encoding.eq_ignore_ascii_case("utf8");

    if is_utf8_enc {
        for (i, result) in (0..).zip(reader.lines()) {
            let line = match result {
                Ok(l) => l,
                Err(_) => return Ok(total), // 二进制文件静默退出
            };
            let line_num = i + 1;
            if emit_if_match(path, &line, line_num, target, regex, cli, on_match) {
                total += 1;
            }
        }
    } else {
        // 非 UTF-8：读整个文件解码后再逐行处理
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        let content = encoding::decode_bytes(&bytes, &cli.input_encoding)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        for (i, line) in content.lines().enumerate() {
            let line_num = i as u32 + 1;
            if emit_if_match(path, line, line_num, target, regex, cli, on_match) {
                total += 1;
            }
        }
    }

    Ok(total)
}

/// 判断一行是否匹配，匹配则调用 on_match(source_name, line_num, text)，返回 true
fn emit_if_match<F>(
    source_name: &str,
    line: &str,
    line_num: u32,
    target: &str,
    regex: Option<&Regex>,
    cli: &Cli,
    on_match: &mut F,
) -> bool
where
    F: FnMut(&str, u32, &str),
{
    let matched = line_matches(line, target, cli.ignore_case, regex);

    // -x
    let matched = if !cli.extended_regexp && cli.line_regexp {
        if cli.ignore_case { line.to_lowercase() == target.to_lowercase() }
        else { line == target }
    } else { matched };

    // -w
    let matched = if !cli.extended_regexp && cli.word_regexp && matched {
        let t = if cli.ignore_case { &target.to_lowercase() } else { target };
        let s = if cli.ignore_case { line.to_lowercase() } else { line.to_string() };
        s.match_indices(t)
            .any(|(pos, m)| is_word_boundary(&s, pos) && is_word_boundary(&s, pos + m.len()))
    } else { matched };

    if matched {
        let text = highlight_line(line, target, cli.ignore_case, regex);
        on_match(source_name, line_num, &text);
    }
    matched
}

// ============================================================
// 搜索入口（全量内存版，用于 -v / --all / 上下文）
// ============================================================

/// 搜索匹配行（或反向匹配），返回 (来源, 行号, 内容）
/// 内部调用 get_content 读取数据
pub fn search(cli: &Cli) -> Vec<(String, u32, String)> {
    let sources = get_content(cli).unwrap_or_else(|e| {
        eprintln!("读取错误: {e}");
        std::process::exit(1);
    });
    search_from(cli, &sources)
}

/// 在已有的 sources 上执行搜索，不重复读取
pub fn search_from(cli: &Cli, sources: &[(String, String)]) -> Vec<(String, u32, String)> {
    let regex = compile_regex(cli).unwrap_or_else(|e| {
        eprintln!("正则错误: {e}");
        std::process::exit(1);
    });

    let target = &cli.target;

    sources
        .par_iter()
        .flat_map_iter(|(source_name, content)| {
            let source_name = source_name.clone();
            let re = regex.clone();
            content.lines().enumerate().filter_map(move |(i, line)| {
                let line_num = i as u32 + 1;
                let matched = line_matches(line, target, cli.ignore_case, re.as_ref());

                // -x 整行匹配（固定字符串模式）
                let matched = if !cli.extended_regexp && cli.line_regexp {
                    if cli.ignore_case {
                        line.to_lowercase() == target.to_lowercase()
                    } else {
                        line == target
                    }
                } else {
                    matched
                };

                // -w 整词匹配（固定字符串模式）
                let matched = if !cli.extended_regexp && cli.word_regexp && matched {
                    let t = if cli.ignore_case { &target.to_lowercase() } else { target };
                    let s = if cli.ignore_case { line.to_lowercase() } else { line.to_string() };
                    s.match_indices(t)
                        .any(|(pos, m)| is_word_boundary(&s, pos) && is_word_boundary(&s, pos + m.len()))
                } else {
                    matched
                };

                let should_include = if cli.invert_match { !matched } else { matched };

                should_include.then(|| {
                    let text = if matched {
                        highlight_line(line, target, cli.ignore_case, re.as_ref())
                    } else {
                        line.to_string()
                    };
                    (source_name.clone(), line_num, text)
                })
            })
        })
        .collect()
}
