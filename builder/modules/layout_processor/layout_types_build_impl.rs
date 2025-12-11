use super::*;

impl IdString {
    /// 编译期创建ID字符串（带长度校验）
    pub fn new(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            return Err("ID不能为空".to_string());
        }
        if s.len() > MAX_ID_LENGTH {
            return Err(format!("ID长度超限: {} > {}", s.len(), MAX_ID_LENGTH));
        }
        Ok(Self(s.to_string()))
    }
}

impl ContentString {
    /// 编译期创建内容字符串（带长度+非空校验）
    pub fn new(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            return Err("内容不能为空".to_string());
        }
        if s.len() > MAX_CONTENT_LENGTH {
            return Err(format!(
                "内容长度超限: {} > {}",
                s.len(),
                MAX_CONTENT_LENGTH
            ));
        }
        Ok(Self(s.to_string()))
    }
}

impl ConditionString {
    pub fn new(s: &str) -> Result<Self, String> {
        if s.len() > MAX_CONDITION_LENGTH {
            return Err(format!(
                "条件长度超限: {} > {}",
                s.len(),
                MAX_CONDITION_LENGTH
            ));
        }
        Ok(Self(s.to_string()))
    }
}

impl LayoutPool {
    /// 创建空布局池
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root_node_id: 0,
            id_map: Vec::new(),
        }
    }

    /// 编译期添加节点到池（自动生成NodeId）
    /// 嵌入式适配：顺序插入，无哈希计算
    pub fn add_node(&mut self, node: LayoutNode) -> Result<NodeId, String> {
        let node_id = self.nodes.len() as NodeId;
        let id = match &node {
            LayoutNode::Container(c) => c.id.clone(),
            LayoutNode::Text(t) => t.id.clone(),
            LayoutNode::Icon(i) => i.id.clone(),
            LayoutNode::Line(l) => l.id.clone(),
            LayoutNode::Rectangle(r) => r.id.clone(),
            LayoutNode::Circle(c) => c.id.clone(),
        };

        // 编译期校验ID唯一性（顺序表遍历）
        if self
            .id_map
            .iter()
            .any(|(existing_id, _)| existing_id == &id)
        {
            return Err(format!("ID重复: {}", id.as_str()));
        }

        self.nodes.push(node);
        self.id_map.push((id, node_id));
        Ok(node_id)
    }
}
