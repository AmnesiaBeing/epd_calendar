//! 跨平台延时函数

use std::time::Duration;

/// 纳秒级延时
pub fn delay_ns(nanos: u64) {
    #[cfg(feature = "pc")]
    {
        // PC端使用精确延时
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_nanos(nanos) {}
    }

    #[cfg(feature = "embedded")]
    {
        // 嵌入式Linux使用libc的nanosleep
        use libc::{nanosleep, timespec};
        let sec = nanos / 1_000_000_000;
        let nsec = (nanos % 1_000_000_000) as i32;

        let req = timespec {
            tv_sec: sec as i64,
            tv_nsec: nsec,
        };
        let mut rem = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        unsafe {
            nanosleep(&req, &mut rem);
        }
    }
}

/// 毫秒级延时
pub fn delay_ms(millis: u64) {
    std::thread::sleep(Duration::from_millis(millis));
}
