// 线性同余发生器（LCG）实现
pub struct Lcg {
    state: u32,
}

impl Lcg {
    // 初始化：根据平台获取随机种子
    pub fn new() -> Self {
        let seed = Self::get_random_seed();
        log::info!("LCG initialized with seed: {}", seed);
        Lcg { state: seed }
    }

    // 从外部提供种子初始化
    #[allow(dead_code)]
    pub fn with_seed(seed: u32) -> Self {
        log::info!("LCG initialized with provided seed: {}", seed);
        Lcg { state: seed }
    }

    // 平台相关的随机种子获取
    #[cfg(any(feature = "embedded_linux", feature = "simulator"))]
    fn get_random_seed() -> u32 {
        use embassy_time::Instant;

        // 使用 embassy-time 获取高精度时间作为种子
        let now = Instant::now();
        let duration = now.as_micros();

        // 混合使用微秒和纳秒来增加随机性
        let nanos = (duration % 1_000_000) as u32;
        let micros = (duration / 1_000) as u32;

        (micros.wrapping_mul(1103515245) ^ nanos).wrapping_add(12345)
    }

    // 默认回退实现
    #[cfg(not(any(feature = "embedded_linux", feature = "simulator",)))]
    fn get_random_seed() -> u32 {
        log::warn!("Unknown platform, using fixed seed");
        134512344
    }

    // 生成下一个随机数（32位）
    pub fn next(&mut self) -> u32 {
        const A: u32 = 1103515245; // LCG乘数（glibc标准）
        const C: u32 = 12345; // 增量
        self.state = A.wrapping_mul(self.state).wrapping_add(C);
        self.state
    }

    // 生成0..max-1的随机索引
    pub fn next_index(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.next() as usize) % max
    }
}
