use crate::types::Move;

pub struct NodeTable {
    table: Box<[[u64; 64]; 64]>,
}

impl NodeTable {
    pub fn get(&self, mv: Move) -> u64 {
        self.table[mv.start()][mv.target()]
    }

    pub fn add(&mut self, mv: Move, nodes: u64) {
        self.table[mv.start()][mv.target()] += nodes;
    }
}

impl Default for NodeTable {
    fn default() -> Self {
        Self {
            table: Box::new([[0; 64]; 64]),
        }
    }
}
