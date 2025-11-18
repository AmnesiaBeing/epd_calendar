// 线性同余发生器（LCG）实现
pub struct Lcg {
    state: u32,
}

impl Lcg {
    // 初始化：需要动态种子（关键！）
    pub fn new() -> Self {
        Lcg { state: 2123451234 } // 这里的种子是随便写的，后续应该修改成通过ADC获取
    }

    // 生成下一个随机数（32位）
    pub fn next(&mut self) -> u32 {
        const A: u32 = 1103515245; // LCG乘数（glibc标准）
        const C: u32 = 12345;      // 增量
        self.state = A.wrapping_mul(self.state).wrapping_add(C);
        self.state
    }

    // 生成0..max-1的随机索引
    pub fn next_index(&mut self, max: usize) -> usize {
        (self.next() as usize) % max
    }
}
