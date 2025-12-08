// src/driver/rng.rs

/// 本模块实现了`getrandom` crate的自定义随机数生成器接口
/// 当在ESP32平台上使用`getrandom` crate时，会自动调用本模块的实现
/// 具体可查看：https://docs.rs/getrandom/latest/getrandom/#custom-backend

#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_v03_custom(
    dest: *mut u8,
    len: usize,
) -> Result<(), getrandom::Error> {
    unsafe { esp_hal::rng::Rng::new().read_into_raw(dest, len) };
    Ok(())
}
