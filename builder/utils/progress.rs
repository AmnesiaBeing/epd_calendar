use std::time::Instant;

/// 进度跟踪器
pub struct ProgressTracker {
    start_time: Instant,
    current_stage: Option<String>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            current_stage: None,
        }
    }

    pub fn start_stage(&mut self, name: &str) {
        self.current_stage = Some(name.to_string());
        println!("cargo:warning=🚀 开始: {}", name);
    }

    pub fn complete_stage(&mut self) {
        if let Some(stage) = &self.current_stage {
            println!("cargo:warning=✅ 完成: {}", stage);
        }
    }

    pub fn update_progress(&self, current: usize, total: usize, operation: &str) {
        let percentage = (current as f32 / total as f32 * 100.0) as usize;
        println!(
            "cargo:warning=📊 {}: {}/{} ({}%)",
            operation, current, total, percentage
        );
    }

    pub fn finish_build(&self) {
        let duration = self.start_time.elapsed();
        println!("cargo:warning=🎉 构建完成! 耗时: {:.2?}", duration);
    }
}
