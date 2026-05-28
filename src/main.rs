use clap::Parser;
use grep_redo::cli::Cli;
use grep_redo::engine;

fn main() {
    let cli = Cli::parse();

    // 配置 rayon 线程池
    if cli.threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(cli.threads)
            .build_global()
            .unwrap();
    }

    engine::run(&cli);
}
