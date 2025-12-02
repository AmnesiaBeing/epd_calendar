use getrandom::Error;

#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_v03_custom(dest: *mut u8, len: usize) -> Result<(), Error> {
    unsafe { esp_hal::rng::Rng::new().read_into_raw(dest, len) };
    Ok(())
}
