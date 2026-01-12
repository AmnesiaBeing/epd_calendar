
pub type LxxAsyncMutex<T> = embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, T>;
