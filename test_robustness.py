#!/usr/bin/env python3
"""
grep_redo 鲁棒性测试
用法: python test_robustness.py [--build]
"""

import subprocess
import sys
import os
import shlex

PASS = 0
FAIL = 0
total = 0

BIN = "cargo run --"


def setup_module():
    files = {}

    with open("test_robust.txt", "w", encoding="utf-8") as f:
        f.write("hello world\n")
        f.write("TARGET here\n")
        f.write("target here\n")
        f.write("another TaRgEt\n")
        f.write("  target with spaces\n")
        f.write("target at start\n")
        f.write("line with multiple target in one target line\n")
        f.write("no match here\n")
        f.write("last line\n")

    with open("test_empty.txt", "w", encoding="utf-8") as f:
        pass

    with open("test_unicode.txt", "w", encoding="utf-8") as f:
        f.write("Hello 世界\n")
        f.write("Rust 编程语言\n")
        f.write("正则表达式测试\n")

    with open("test_single_line.txt", "w", encoding="utf-8") as f:
        f.write("this is a single line with target in it")

    with open("test_long_line.txt", "w", encoding="utf-8") as f:
        f.write("A" * 10000 + " target " + "B" * 10000 + "\n")
        f.write("normal line\n")

    with open("test_gbk_robust.txt", "w", encoding="gbk") as f:
        f.write("Hello 世界 GBK\n")
        f.write("target line\n")

    files.update({
        "basic": "test_robust.txt",
        "empty": "test_empty.txt",
        "unicode": "test_unicode.txt",
        "single": "test_single_line.txt",
        "long": "test_long_line.txt",
        "gbk": "test_gbk_robust.txt",
    })
    return files


def run(args):
    # Windows CMD 只认双引号
    def q(s):
        return f'"{s}"' if (" " in s or "|" in s or "^" in s or "$" in s) else s
    cmd = f'{BIN} {" ".join(q(a) for a in args)}'
    r = subprocess.run(
        cmd, capture_output=True, shell=True, text=True, timeout=30
    )
    stderr = "\n".join(
        line for line in r.stderr.splitlines()
        if not any(w in line for w in ["warning:", "Compiling", "Finished",
                                        "Running", "Blocking", "Downloading",
                                        "Locking", "Adding", "Checking",
                                        "Downloaded"])
    )
    return r.stdout, stderr, r.returncode


def check(name, condition, detail=""):
    global PASS, FAIL, total
    total += 1
    if condition:
        PASS += 1
        print(f"  ✅ {name}")
    else:
        FAIL += 1
        msg = f"  ❌ {name}"
        if detail:
            msg += f" — {detail}"
        print(msg)


def run_and_check(name, args, expect_match=True, expect_error=False):
    stdout, stderr, code = run(args)
    has_output = bool(stdout.strip())

    if expect_error:
        check(name, code != 0 or bool(stderr.strip()),
              f"期望报错但 code={code} stderr={stderr[:100]}")
    elif expect_match:
        check(name, has_output,
              f"期望匹配但无输出, stdout=({stdout[:80]})")
    else:
        check(name, not has_output,
              f"期望无匹配但有输出: ({stdout[:80]})")
    return stdout, stderr, code


# ─── 测试用例 ───

def test_basic_matching(files):
    print("\n───── 基础匹配 ─────")
    f = files["basic"]
    run_and_check("精确匹配", ["target", f])
    run_and_check("不匹配", ["zzzznotexist", f], expect_match=False)
    run_and_check("行首匹配", ["target at start", f])
    run_and_check("最后一行匹配", ["last line", f])
    run_and_check("一行多次匹配", ["target", f])

    stdout, _, _ = run(["nope", f])
    check("无匹配时输出为空", stdout.strip() == "")


def test_case_insensitive(files):
    print("\n───── 忽略大小写 -i ─────")
    f = files["basic"]
    stdout, _, _ = run(["-i", "target", f])
    lines = [l for l in stdout.splitlines() if l.strip()]
    check("-i 匹配所有大小写变体", len(lines) == 6, f"期望6行, 得到{len(lines)}行")


def test_regex(files):
    print("\n───── 正则 -E ─────")
    f = files["basic"]
    run_and_check(". 通配", ["-E", "t..get", f])
    run_and_check("行首 ^", ["-E", "^target", f])
    run_and_check("行尾 $", ["-E", "here$", f])
    run_and_check("| 或", ["-E", "hello|target", f])

    _, stderr, _ = run(["-E", "*bad", f])
    check("无效正则报错", "正则错误" in stderr or "error" in stderr.lower(),
          f"stderr={stderr[:80]}")


def test_invert_match(files):
    print("\n───── 反向匹配 -v ─────")
    f = files["basic"]
    stdout, _, _ = run(["-v", "target", f])
    lines = [l for l in stdout.splitlines() if l.strip()]
    # 全文9行, 精确target命中4行 → -v 应5行
    check("-v 行数正确", len(lines) == 5, f"期望5, 得到{len(lines)}")
    check("-v 不含匹配词", all("target" not in l for l in lines),
          f"含 target: {lines}")


def test_line_number(files):
    print("\n───── 行号 -n ─────")
    f = files["basic"]
    stdout, _, _ = run(["-n", "target", f])
    lines = [l for l in stdout.splitlines() if l.strip()]
    check("-n 有行号", all(":" in l for l in lines), f"缺冒号: {lines}")
    check("-n 行号从1开始", any(l.startswith("3:") for l in lines),
          f"缺 '3:': {lines}")


def test_count(files):
    print("\n───── 计数 -c ─────")
    f = files["basic"]
    stdout, _, _ = run(["-c", "target", f])
    check("-c 计数值", stdout.strip() == "4", f"期望4, 得到{stdout.strip()}")


def test_all_mode(files):
    print("\n───── 全文 --all ─────")
    f = files["basic"]
    stdout, _, _ = run(["--all", "target", f])
    lines = [l for l in stdout.splitlines() if l.strip()]
    check("--all 显示所有行", len(lines) == 9, f"期望9, 得到{len(lines)}")
    check("--all 含高亮标记", "\x1b[31m" in stdout, f"缺高亮标记")


def test_context(files):
    print("\n───── 上下文 -C ─────")
    f = files["basic"]
    stdout, _, _ = run(["-C", "1", "target", f])
    lines = [l for l in stdout.splitlines() if l.strip()]
    # target 在第3行 → +-1 → 第2,3,4 至少
    check("-C 1 多行", len(lines) > 1, f"仅1行: {lines}")
    check("-C 1 含上文", any("TARGET" in l for l in lines), f"缺上文")


def test_empty_file(files):
    print("\n───── 空文件 ─────")
    f = files["empty"]
    stdout, _, code = run(["anything", f])
    check("空文件无输出", stdout.strip() == "")
    check("空文件退出码1", code == 1, f"code={code}")


def test_nonexistent_file(files):
    print("\n───── 文件不存在 ─────")
    _, stderr, _ = run(["x", "no_such_file_xyz.txt"])
    check("报错提示", bool(stderr.strip()), f"stderr={stderr[:80]}")


def test_unicode_matching(files):
    print("\n───── Unicode ─────")
    f = files["unicode"]
    run_and_check("中文匹配", ["世界", f])
    run_and_check("中文不匹配", ["不存在", f], expect_match=False)


def test_long_line(files):
    print("\n───── 超长行 ─────")
    f = files["long"]
    stdout, _, _ = run(["target", f])
    check("超长行能匹配", "target" in stdout, f"无输出")


def test_single_line_file(files):
    print("\n───── 单行文件 ─────")
    f = files["single"]
    stdout, _, _ = run(["target", f])
    check("单行文件匹配", "target" in stdout, f"无输出")


def test_exit_code(files):
    print("\n───── 退出码 ─────")
    f = files["basic"]
    _, _, c1 = run(["target", f])
    check("有匹配 code=0", c1 == 0, f"code={c1}")
    _, _, c2 = run(["zzzzzznonexist", f])
    check("无匹配 code=1", c2 == 1, f"code={c2}")


def test_multi_file(files):
    print("\n───── 多文件 ─────")
    f1, f2 = files["basic"], files["unicode"]
    stdout, _, _ = run(["target", f1, f2])
    check("多文件有输出", bool(stdout.strip()), "无输出")
    check("多文件含文件名", f1 in stdout, f"缺文件名")


def test_recursive(files):
    print("\n───── 递归 -r ─────")
    stdout, _, _ = run(["-n", "target", "-r", "--include", "*.txt", "."])
    check("-r 有输出", bool(stdout.strip()), "无输出")


def test_threads(files):
    print("\n───── 线程数 -j ─────")
    f = files["basic"]
    for j in [1, 2, 4]:
        stdout, _, _ = run(["-j", str(j), "target", f])
        check(f"-j {j} 正常", "target" in stdout, f"无输出")


def test_quiet(files):
    print("\n───── 静默 -q ─────")
    f = files["basic"]
    stdout, _, c1 = run(["-q", "target", f])
    check("-q 有匹配无输出", stdout.strip() == "")
    check("-q 有匹配 code=0", c1 == 0, f"code={c1}")

    _, _, c2 = run(["-q", "zzzznone", f])
    check("-q 无匹配 code=1", c2 == 1, f"code={c2}")


def test_files_with_matches(files):
    print("\n───── 文件名 -l ─────")
    f1, f2 = files["basic"], files["unicode"]
    stdout, _, _ = run(["-l", "target", f1, f2])
    check("-l 输出文件名", "test_robust" in stdout, f"输出: {stdout[:80]}")
    check("-l 不输出无匹配文件", "test_unicode" not in stdout,
          f"误出现: {stdout[:80]}")


def test_encoding(files):
    print("\n───── 编码 ─────")
    f = files["gbk"]
    stdout, _, _ = run(["--input-encoding", "gbk", "target", f])
    check("GBK 可匹配", "target" in stdout, f"失败")

    _, stderr, _ = run(["target", f])
    check("不指定编码时报错", bool(stderr.strip()), f"stderr={stderr[:80]}")


def test_binary_skip(files):
    print("\n───── 二进制 ─────")
    with open("test_binary.bin", "wb") as f:
        f.write(b"\x00\x01\x02\xff\xfetarget\x00\xff")

    stdout, stderr, _ = run(["target", "test_binary.bin"])
    check("二进制无输出", stdout.strip() == "", f"有输出: {stdout[:50]}")
    check("二进制不崩溃", "panic" not in stderr.lower(),
          f"stderr={stderr[:100]}")


def test_regex_context(files):
    print("\n───── 正则+上下文 ─────")
    f = files["basic"]
    stdout, _, _ = run(["-E", "-i", "-C", "1", "t..get", f])
    lines = [l for l in stdout.splitlines() if l.strip()]
    check("有输出", len(lines) > 0, "无输出")
    check("多行", len(lines) >= 3, f"仅{len(lines)}行")


def test_help(files):
    print("\n───── --help ─────")
    stdout, _, _ = run(["--help"])
    check("--help 有内容", len(stdout) > 100, f"仅{len(stdout)}字符")
    check("含 --threads", "--threads" in stdout)
    check("含 -E", "-E" in stdout or "--extended-regexp" in stdout)


def test_stdin_mode(files):
    """通过 stdin 输入（需要模拟管道）"""
    print("\n───── stdin ─────")
    r = subprocess.run(
        f'{BIN} target',
        input=b"hello target world\nno match\nanother target\n",
        capture_output=True, shell=True, text=False, timeout=15
    )
    out = r.stdout.decode("utf-8", errors="replace")
    check("stdin 匹配", "target" in out, f"无输出: {out[:50]}")
    check("stdin 不匹配行不出现", "no match" not in out,
          f"误出: {out[:50]}")


# ─── 主入口 ───

def run_all():
    files = setup_module()

    print("Building...")
    r = subprocess.run("cargo build 2>&1", shell=True,
                       capture_output=True, text=True, timeout=120)
    if r.returncode != 0:
        print("❌ 编译失败:", r.stderr[:500])
        sys.exit(1)
    print("✅ 编译成功\n")

    tests = [
        ("基础匹配", test_basic_matching),
        ("忽略大小写", test_case_insensitive),
        ("正则", test_regex),
        ("反向匹配", test_invert_match),
        ("行号", test_line_number),
        ("计数", test_count),
        ("全文", test_all_mode),
        ("上下文", test_context),
        ("空文件", test_empty_file),
        ("文件不存在", test_nonexistent_file),
        ("Unicode", test_unicode_matching),
        ("超长行", test_long_line),
        ("单行文件", test_single_line_file),
        ("退出码", test_exit_code),
        ("多文件", test_multi_file),
        ("递归", test_recursive),
        ("线程数", test_threads),
        ("静默", test_quiet),
        ("文件名", test_files_with_matches),
        ("编码", test_encoding),
        ("二进制", test_binary_skip),
        ("正则+上下文", test_regex_context),
        ("--help", test_help),
        ("stdin", test_stdin_mode),
    ]

    for name, func in tests:
        print(f"── {name} ──")
        try:
            func(files)
        except Exception as e:
            global FAIL, total
            total += 1
            FAIL += 1
            import traceback
            print(f"  ❌ 异常: {e}")
            traceback.print_exc()

    for fname in ["test_robust.txt", "test_empty.txt", "test_unicode.txt",
                  "test_single_line.txt", "test_long_line.txt",
                  "test_gbk_robust.txt", "test_binary.bin"]:
        if os.path.exists(fname):
            os.remove(fname)

    print(f"\n{'='*40}")
    print(f"  总计: {total}")
    print(f"  通过: {PASS}  ✅")
    print(f"  失败: {FAIL}  ❌")
    print(f"{'='*40}")
    return FAIL == 0


if __name__ == "__main__":
    success = run_all()
    sys.exit(0 if success else 1)
