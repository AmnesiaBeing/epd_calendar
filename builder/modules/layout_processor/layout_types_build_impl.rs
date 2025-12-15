use super::*;

impl LayoutPool {
    /// 创建空布局池
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root_node_id: 0,
        }
    }

    /// 编译期添加节点到池（自动生成NodeId）
    /// 嵌入式适配：顺序插入，无哈希计算
    pub fn add_node(&mut self, node: LayoutNode) -> Result<NodeId, String> {
        let node_id = self.nodes.len() as NodeId;
        self.nodes.push(node);
        Ok(node_id)
    }
}
