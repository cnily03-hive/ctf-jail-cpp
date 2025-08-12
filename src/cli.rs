use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "jailbox")]
#[command(about = "CTF沙箱执行环境的Web UI")]
#[command(version = "0.1.0")]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 启动Web服务器
    Listen {
        /// 服务器端口
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// 服务器主机地址
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        /// 上下文目录路径
        #[arg(short, long, default_value = "./context")]
        context: PathBuf,

        /// Rune脚本文件路径
        #[arg(short, long)]
        exec: Option<PathBuf>,
    },
    /// 运行collect函数并返回结果
    Collect {
        /// Rune脚本文件路径
        #[arg(short, long)]
        exec: Option<PathBuf>,

        /// 上下文目录路径
        #[arg(short, long, default_value = "./context")]
        context: PathBuf,

        /// 是否解析 JSON 输出
        #[arg(short = 'P', long, default_value = "false")]
        parse: bool,
    },
    /// 运行check函数并返回结果
    Check {
        /// Rune脚本文件路径
        #[arg(short, long)]
        exec: Option<PathBuf>,

        /// 用户输入
        #[arg(short, long, alias = "user-input")]
        input: String,

        /// 上下文目录路径
        #[arg(short, long, default_value = "./context")]
        context: PathBuf,

        /// 是否解析 JSON 输出
        #[arg(short = 'P', long, default_value = "false")]
        parse: bool,
    },
}
