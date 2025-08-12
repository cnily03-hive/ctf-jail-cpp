use anyhow::{Error, Result};
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, Source, Sources, Value, Vm};
use std::{path::Path, sync::Arc};

pub struct RuneEngine {
    script_path: String,
    data_directory: String,
}

impl RuneEngine {
    pub async fn new(script_path: &Path, data_directory: &Path) -> Result<Self> {
        let script_path_str = script_path.to_string_lossy().to_string();
        let data_directory_str = data_directory.to_string_lossy().to_string();

        Ok(Self {
            script_path: script_path_str,
            data_directory: data_directory_str,
        })
    }

    fn compile_vm(&self) -> Result<Vm> {
        let mut rune_context = rune::Context::with_default_modules()?;

        rune_context.install(super::modules::context::module(true)?)?;

        // Compile script
        let mut sources = Sources::new();
        let mut diagnostics = Diagnostics::new();
        sources.insert(Source::from_path(&self.script_path)?)?;

        let unit = rune::prepare(&mut sources)
            .with_context(&rune_context)
            .with_diagnostics(&mut diagnostics)
            .build();

        if !diagnostics.is_empty() {
            let mut writer = StandardStream::stderr(ColorChoice::Always);
            diagnostics.emit(&mut writer, &sources)?;
        }

        let unit = unit?;
        let runtime = rune_context.runtime()?;
        Ok(Vm::new(Arc::new(runtime), Arc::new(unit)))
    }

    pub async fn call_collect(&self) -> Result<Result<String, String>> {
        let mut vm = self.compile_vm()?;
        let ctx = super::modules::context::Context::new(self.data_directory.clone());
        let output = vm.call(["collect"], (ctx,))?;
        self.process_result(output)
    }

    pub async fn call_check(&self, user_input: &str) -> Result<Result<String, String>> {
        let mut vm = self.compile_vm()?;
        let ctx = super::modules::context::Context::new(self.data_directory.clone());
        let output = vm.call(["check"], (ctx, user_input))?;
        self.process_result(output)
    }

    fn process_result(&self, value: Value) -> Result<Result<String, String>> {
        // Try to extract value from Result type
        match rune::from_value::<Result<Value, Value>>(value.clone()) {
            // rune returns Result
            Ok(result) => match result {
                // rune script successfully returns a success value
                Ok(success_value) => Ok(Ok(rune_value_throw_or_stringify(success_value)?)),
                // rune script successfully returns a error value
                Err(error_value) => Ok(match rune::from_value::<String>(&error_value) {
                    Ok(error_msg) => Err(error_msg),
                    Err(_) => Err(rune_value_throw_or_stringify(error_value)?),
                }),
            },
            // rune returns non Result, treat it as a success returned value
            Err(_) => Ok(Ok(rune_value_throw_or_stringify(value)?)),
        }
    }
}

/// Convert Rune Value to JSON string
fn rune_value_throw_or_stringify(value: Value) -> Result<String> {
    // If it's an Error object, throw runtime exception
    if let Ok(e) = rune::from_value::<anyhow::Error>(value.clone()) {
        return Err(e);
    }
    if let Ok(e) = rune::from_value::<std::io::Error>(value.clone()) {
        return Err(e.into());
    }
    // Try to convert Rune Value to serde_json::Value
    match rune::to_value(&value) {
        Ok(json_value) => {
            // Convert to JSON string
            serde_json::to_string(&json_value).map_err(|e| {
                anyhow::anyhow!("Unable to serialize value: {:?}, error: {}", value, e)
            })
        }
        Err(e) => {
            // direct throw runtime error
            Err(e.into())
        }
    }
}
