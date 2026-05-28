//这是一个重构的grep

// @模块：controller
// @常量：DEFAULT - 默认控制器标志位，值为0
// @常量：CHECK_UPPER - 检查大写字母的控制器标志位，值为1左移0位（即1）
// @常量：SHOW_LINE - 显示target所在行的内容的控制器标志位，值为1左移1位（即2）
// @常量：SHOW_ALL - 在所有内容中高亮的控制器标志位，值为1左移2位（即4）
use std::io;

pub mod Controller{
    pub const DEFAULT: u8 = 0;
    pub const CHECK_UPPER: u8 = 1 << 0;
    pub const SHOW_LINE: u8 = 1 << 1;
    pub const SHOW_ALL: u8 = 1 << 2;
}

pub enum InputSource {
    Stdin,
    File(String),
    Files(Vec<String>),
}

// @结构体：Config
// @字段：filesource: InputSource - 输入源，可以是标准输入或文件输入
// @字段：controller: u8 - 控制器标志位 
// @字段：target: String - 目标字符串
pub struct Config{
    filesource: InputSource,
    controller: u8,
    target: String,
}

//读取目标输入
pub mod Buf{
    use std::io::Read;
    use std::fs;
    use super::*;
    pub fn read(config: &Config) -> io::Result<Vec<(String, String)>> {
        match &config.filesource {
            InputSource::Stdin => {
                let mut buffer = String::new();
                std::io::stdin().read_to_string(&mut buffer)?;
                Ok(vec![("stdin".to_string(), buffer)])
            },
            InputSource::File(filename) => {
                let content = fs::read_to_string(filename)?;
                Ok(vec![(filename.clone(), content)])
            },
            InputSource::Files(files) => {
                let mut sources = Vec::with_capacity(files.len());
                for f in files {
                    let content = fs::read_to_string(f)?;
                    sources.push((f.clone(), content));
                }
                Ok(sources)
            },
        }
    }
}

impl Config {
    // @参数：args: 命令行参数列表
    // @返回值：Result<Config, &'static str> - 成功时返回Config实例，失败时返回错误信息
    // @功能：解析命令行参数并创建Config实例
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        let mut controller = Controller::DEFAULT;
        let mut arg: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
        arg.retain(|&op| {
            match op {
                "-u" | "--upper" => {
                    controller |= Controller::CHECK_UPPER;
                    false
                },
                "-l" | "--line" => {
                    controller |= Controller::SHOW_LINE;
                    false
                },
                "-a" | "--all" => {
                    controller |= Controller::SHOW_ALL;
                    false
                },
                _ => true,
            }
        }); 
        match arg.len() {
            0 => Err("No target specified"),
            1 => Ok(Config {
                filesource: InputSource::Stdin,
                controller,
                target: arg[0].to_string(),
            }),
            2 => {
            let path = arg[1].to_string();
            if path.contains('*') || path.contains('?') || path.contains('[') {
                let entries = glob::glob(&path)
                    .map_err(|_| "Invalid glob pattern")?
                    .filter_map(Result::ok)
                    .collect::<Vec<_>>();
                if entries.is_empty() {
                    return Err("No files matched the glob pattern");
                }
                let paths: Vec<String> = entries.iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect();
                Ok(Config {
                    filesource: if paths.len() == 1 {
                        InputSource::File(paths.into_iter().next().unwrap())
                    } else {
                        InputSource::Files(paths)
                    },
                    controller,
                    target: arg[0].to_string(),
                })
            } else {
                Ok(Config {
                    filesource: InputSource::File(path),
                    controller,
                    target: arg[0].to_string(),
                })
            }
        },
            _ => Err("Too many arguments"),
        }

    }
}

pub mod search {
    use super::*;

    /// 高亮文本中所有匹配 target 的位置
    fn highlight(text: &str, target: &str, check_upper: bool) -> String {
        if check_upper {
            let upper_text = text.to_uppercase();
            let upper_target = target.to_uppercase();
            let matches: Vec<(usize, usize)> = upper_text
                .match_indices(&upper_target)
                .map(|(start, m)| (start, start + m.len()))
                .collect();
            let mut result = text.to_string();
            for &(start, end) in matches.iter().rev() {
                result.replace_range(
                    start..end,
                    &format!("\x1b[31m{}\x1b[0m", &text[start..end]),
                );
            }
            result
        } else {
            text.replace(&target, &format!("\x1b[31m{}\x1b[0m", &target))
        }
    }

    /// 只保留包含 target 的行，其余行删除
    fn keep_matching_lines(content: &str, target: &str, check_upper: bool) -> String {
        content
            .lines()
            .filter(|line| {
                if check_upper {
                    line.to_uppercase().contains(&target.to_uppercase())
                } else {
                    line.contains(target)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 给每行添加行号（从 1 开始）
    fn number_lines(content: &str) -> String {
        content
            .lines()
            .enumerate()
            .map(|(i, line)| format!("{}:{}", i + 1, line))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 执行搜索（主入口）
    pub fn run(config: Config) -> io::Result<()> {
        let target = config.target.clone();
        let sources = Buf::read(&config)?;
        let multi_file = sources.len() > 1;

        let show_line = config.controller & Controller::SHOW_LINE != 0;
        let show_all = config.controller & Controller::SHOW_ALL != 0;
        let check_upper = config.controller & Controller::CHECK_UPPER != 0;

        for (source_name, content) in &sources {
            // 第一步：高亮匹配
            // 第二步：删掉无匹配行（仅 -l / 默认模式）
            // 第三步：加行号（仅 -l）
            let result = if show_all {
                let mut r = highlight(content, &target, check_upper);
                if show_line {
                    r = number_lines(&r);
                }
                r
            } else {
                let mut r = keep_matching_lines(content, &target, check_upper);
                r = highlight(&r, &target, check_upper);
                if show_line {
                    r = number_lines(&r);
                }
                r
            };

            for line in result.lines() {
                if multi_file {
                    println!("{source_name}:{line}");
                } else {
                    println!("{line}");
                }
            }
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new() {
        let args = vec![
            "program".to_string(),
            "-u".to_string(),
            "target".to_string(),
            "output.txt".to_string(),
        ];
        let config = Config::new(&args).unwrap();
        assert_eq!(config.controller, Controller::CHECK_UPPER, "Expected controller to have CHECK_UPPER flag set");
        assert_eq!(config.target, "target", "Expected target to be 'target'");
    }

    #[test]
    fn test_config_new_combined_flags() {
        let args = vec![
            "program".to_string(),
            "-l".to_string(),
            "-a".to_string(),
            "target".to_string(),
        ];
        let config = Config::new(&args).unwrap();
        assert_eq!(config.controller, Controller::SHOW_LINE | Controller::SHOW_ALL);
    }

    #[test]
    fn test_Buf_read() {
        let config = Config {
            controller: Controller::DEFAULT,
            target: "target".to_string(),
            filesource: InputSource::File("test_input.txt".to_string()),
        };
        let sources = Buf::read(&config).unwrap();
        assert_eq!(sources.len(), 1, "Expected one source");
        assert_eq!(sources[0].1, "This is a test input file.", "Expected content to match the contents of test_input.txt");
    }
}
