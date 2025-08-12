use anyhow::Result;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Context, Diagnostics, Source, Sources, Value, Vm};
use std::{path::Path, sync::Arc};

pub struct RuneEngine {
    script_path: String,
    context_path: String,
}

impl RuneEngine {
    pub async fn new(script_path: &Path, context_path: &Path) -> Result<Self> {
        let script_path_str = script_path.to_string_lossy().to_string();
        let context_path_str = context_path.to_string_lossy().to_string();

        Ok(Self {
            script_path: script_path_str,
            context_path: context_path_str,
        })
    }

    fn compile_vm(&self) -> Result<Vm> {
        // 设置全局的context路径
        super::modules::context::set_context_path(self.context_path.clone());

        // 创建Rune运行时上下文
        let mut context = Context::with_default_modules()?;

        // 安装我们的自定义模块
        context.install(super::modules::module(true)?)?;

        // 编译脚本
        let mut sources = Sources::new();
        let mut diagnostics = Diagnostics::new();
        sources.insert(Source::from_path(&self.script_path)?)?;

        let unit = rune::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .build();

        if !diagnostics.is_empty() {
            let mut writer = StandardStream::stderr(ColorChoice::Always);
            diagnostics.emit(&mut writer, &sources)?;
        }

        let unit = unit?;
        let runtime = context.runtime()?;
        Ok(Vm::new(Arc::new(runtime), Arc::new(unit)))
    }

    pub async fn call_collect(&self) -> Result<Result<String, String>> {
        let mut vm = self.compile_vm()?;
        let output = vm.call(["collect"], ())?;
        self.process_result(output)
    }

    pub async fn call_check(&self, user_input: &str) -> Result<Result<String, String>> {
        let mut vm = self.compile_vm()?;
        let output = vm.call(["check"], (user_input,))?;
        self.process_result(output)
    }

    fn process_result(&self, value: Value) -> Result<Result<String, String>> {
        // 尝试从Result类型中提取值
        match rune::from_value::<Result<Value, String>>(value.clone()) {
            // rune returns Result
            Ok(result) => match result {
                // rune returns a success value
                Ok(success_value) => Ok(Ok(stringify_rune_value(success_value)?)),
                // rune returns a error value
                Err(error_msg) => Ok(Err(error_msg)),
            },
            // rune returns non Result, treat it as a success returned value
            Err(_) => Ok(Ok(stringify_rune_value(value)?)),
        }
    }
}

/// 将 Rune Value 转换为 JSON 字符串
fn stringify_rune_value(value: Value) -> Result<String> {
    // 尝试将 Rune Value 转换为 serde_json::Value
    match rune::to_value(&value) {
        Ok(json_value) => {
            // 转换为 JSON 字符串
            serde_json::to_string(&json_value)
                .map_err(|e| anyhow::anyhow!("无法序列化值: {:?}, 错误: {}", value, e))
        }
        Err(e) => {
            // 如果无法转换为 JSON，尝试转换为字符串
            match rune::from_value::<String>(value.clone()) {
                Ok(s) => serde_json::to_string(&s)
                    .map_err(|e| anyhow::anyhow!("无法序列化字符串: {}, 错误: {}", s, e)),
                Err(_) => Err(anyhow::anyhow!("无法转换值: {:?}, 错误: {}", value, e)),
            }
        }
    }
}
