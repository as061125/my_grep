use std::io::Write;

use crate::cli::Cli;
use crate::encoding;
use crate::matcher;

// ============================================================
// 上下文行展开 — 在 results 中插入匹配行周围的上下文行
// ============================================================

fn expand_context(
    sources: &[(String, String)],
    results: &[(String, u32, String)],
    before: usize,
    after: usize,
) -> Vec<(String, u32, String)> {
    if before == 0 && after == 0 {
        return results.to_vec();
    }

    // 按源文件分组，对每个文件独立展开
    let mut expanded = Vec::new();

    for (src, content) in sources {
        // 收集该文件中所有匹配行号
        let match_lines: std::collections::BTreeSet<u32> = results
            .iter()
            .filter(|(s, _, _)| s == src)
            .map(|(_, n, _)| *n)
            .collect();

        if match_lines.is_empty() {
            continue;
        }

        // 构建上下文行号集合（含匹配行自身）
        let total_lines = content.lines().count() as u32;
        let mut context_set: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        for &ln in &match_lines {
            let lo = ln.saturating_sub(before as u32);
            let hi = (ln + after as u32).min(total_lines);
            for n in lo..=hi {
                context_set.insert(n);
            }
        }

        // 按行号顺序输出
        for (i, line) in content.lines().enumerate() {
            let line_num = i as u32 + 1;
            if !context_set.contains(&line_num) {
                continue;
            }

            if match_lines.contains(&line_num) {
                // 匹配行 → 取高亮版本
                let hl = results
                    .iter()
                    .find(|(s, n, _)| s == src && *n == line_num)
                    .map(|(_, _, c)| c.as_str())
                    .unwrap_or(line);
                expanded.push((src.clone(), line_num, hl.to_string()));
            } else {
                // 上下文行 → 原文
                expanded.push((src.clone(), line_num, line.to_string()));
            }
        }
    }

    expanded
}

// ============================================================
// 输出上下文
// ============================================================

pub struct OutputContext<'a> {
    pub cli: &'a Cli,
    pub sources: &'a [(String, String)],
    pub results: &'a [(String, u32, String)],
}

impl OutputContext<'_> {
    fn println(&self, text: &str) {
        let bytes = encoding::encode_text(text, &self.cli.output_encoding);
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(&bytes);
        let _ = stdout.write_all(b"\n");
    }
}

// ============================================================
// 输出策略 trait
// ============================================================

pub trait OutputStrategy {
    fn execute(&self, ctx: &OutputContext);
}

// ============================================================
// 各输出模式
// ============================================================

/// 匹配行输出
struct LineOutput;
impl OutputStrategy for LineOutput {
    fn execute(&self, ctx: &OutputContext) {
        let multi_file = ctx.sources.len() > 1;
        for (src, line_num, content) in ctx.results {
            if ctx.cli.line_number {
                if multi_file || ctx.cli.with_filename {
                    ctx.println(&format!("{src}:{line_num}:{content}"));
                } else {
                    ctx.println(&format!("{line_num}:{content}"));
                }
            } else {
                if multi_file || ctx.cli.with_filename {
                    ctx.println(&format!("{src}:{content}"));
                } else {
                    ctx.println(content);
                }
            }
        }
    }
}

/// 全文高亮输出
struct AllOutput;
impl OutputStrategy for AllOutput {
    fn execute(&self, ctx: &OutputContext) {
        let matched_lines: std::collections::BTreeSet<(String, u32)> = ctx
            .results
            .iter()
            .map(|(s, n, _)| (s.clone(), *n))
            .collect();

        for (src, content) in ctx.sources {
            let multi_file = ctx.sources.len() > 1 || ctx.cli.with_filename;
            for (i, line) in content.lines().enumerate() {
                let line_num = i as u32 + 1;
                let displayed = if matched_lines.contains(&(src.clone(), line_num)) {
                    ctx.results
                        .iter()
                        .find(|(s, n, _)| s == src && *n == line_num)
                        .map(|(_, _, c)| c.as_str())
                        .unwrap_or(line)
                } else {
                    line
                };

                if ctx.cli.line_number {
                    if multi_file {
                        ctx.println(&format!("{src}:{line_num}:{displayed}"));
                    } else {
                        ctx.println(&format!("{line_num}:{displayed}"));
                    }
                } else {
                    if multi_file {
                        ctx.println(&format!("{src}:{displayed}"));
                    } else {
                        ctx.println(displayed);
                    }
                }
            }
        }
    }
}

/// 计数
struct CountOutput;
impl OutputStrategy for CountOutput {
    fn execute(&self, ctx: &OutputContext) {
        if ctx.sources.len() == 1 {
            ctx.println(&ctx.results.len().to_string());
        } else {
            let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
            for (src, _, _) in ctx.results {
                *counts.entry(src.as_str()).or_insert(0) += 1;
            }
            for (src, count) in &counts {
                ctx.println(&format!("{src}:{count}"));
            }
        }
    }
}

/// 仅显示有匹配的文件名
struct FilesWithMatchesOutput;
impl OutputStrategy for FilesWithMatchesOutput {
    fn execute(&self, ctx: &OutputContext) {
        let mut seen = std::collections::BTreeSet::new();
        for (src, _, _) in ctx.results {
            if seen.insert(src.as_str()) {
                ctx.println(src);
            }
        }
    }
}

/// 仅显示无匹配的文件名
struct FilesWithoutMatchOutput;
impl OutputStrategy for FilesWithoutMatchOutput {
    fn execute(&self, ctx: &OutputContext) {
        let matched: std::collections::BTreeSet<&str> =
            ctx.results.iter().map(|(s, _, _)| s.as_str()).collect();
        for (src, _) in ctx.sources {
            if !matched.contains(src.as_str()) {
                ctx.println(src);
            }
        }
    }
}

/// 静默
struct QuietOutput;
impl OutputStrategy for QuietOutput {
    fn execute(&self, _ctx: &OutputContext) {}
}

// ============================================================
// 策略选择 + 主入口
// ============================================================

fn choose(cli: &Cli) -> Box<dyn OutputStrategy> {
    if cli.quiet {
        Box::new(QuietOutput)
    } else if cli.files_with_matches {
        Box::new(FilesWithMatchesOutput)
    } else if cli.files_without_match {
        Box::new(FilesWithoutMatchOutput)
    } else if cli.count {
        Box::new(CountOutput)
    } else if cli.all {
        Box::new(AllOutput)
    } else {
        Box::new(LineOutput)
    }
}

/// 流式输出一行（编码 + 换行）
fn write_line(text: &str, encoding: &str) {
    let bytes = crate::encoding::encode_text(text, encoding);
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(&bytes);
    let _ = stdout.write_all(b"\n");
}

/// 流式路径：逐行匹配、逐行输出
fn run_streaming(cli: &Cli) {
    let multi_file = cli.filenames.len() > 1;
    let mut count = 0usize;

    let result = matcher::search_stream(cli, |source: &str, line_num: u32, text: &str| {
        count += 1;

        if cli.quiet {
            return;
        }
        if cli.files_with_matches {
            write_line(source, &cli.output_encoding);
            return;
        }
        if cli.count {
            return;
        }

        let out = if cli.line_number {
            if multi_file || cli.with_filename {
                format!("{source}:{line_num}:{text}")
            } else {
                format!("{line_num}:{text}")
            }
        } else {
            if multi_file || cli.with_filename {
                format!("{source}:{text}")
            } else {
                text.to_string()
            }
        };
        write_line(&out, &cli.output_encoding);
    });

    match result {
        Ok(total) => {
            if cli.count {
                write_line(&total.to_string(), &cli.output_encoding);
            }
            if total == 0 {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("读取错误: {e}");
            std::process::exit(1);
        }
    }
}

/// 全量内存路径：用于 -v / --all / 上下文模式
fn run_memory(cli: &Cli) {
    let sources = matcher::get_content(cli).unwrap_or_else(|e| {
        eprintln!("读取错误: {e}");
        std::process::exit(1);
    });

    let results = matcher::search(cli);

    let before = cli.before_context.unwrap_or_else(|| cli.context.unwrap_or(0));
    let after = cli.after_context.unwrap_or_else(|| cli.context.unwrap_or(0));
    let results = if !cli.all && (before > 0 || after > 0) {
        expand_context(&sources, &results, before, after)
    } else {
        results
    };

    let ctx = OutputContext {
        cli,
        sources: &sources,
        results: &results,
    };

    let strategy = choose(cli);
    strategy.execute(&ctx);
}

/// 主入口
pub fn run(cli: &Cli) {
    let need_full = cli.invert_match
        || cli.all
        || cli.files_with_matches
        || cli.files_without_match
        || cli.before_context.unwrap_or(0) > 0
        || cli.after_context.unwrap_or(0) > 0
        || cli.context.unwrap_or(0) > 0;

    if need_full {
        run_memory(cli);
    } else {
        run_streaming(cli);
    }
}
