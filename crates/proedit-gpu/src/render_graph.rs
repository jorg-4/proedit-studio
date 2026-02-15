//! Render graph with topological sort for compositing order.
//!
//! Each node represents a render operation (blit, effect, composite).
//! Edges represent data dependencies (output of one feeds input of another).
//! The graph is sorted topologically before execution to ensure correct order.

use std::collections::HashMap;

/// Unique identifier for a render node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

/// The type of operation this node performs.
#[derive(Debug, Clone)]
pub enum NodeOp {
    /// Load a texture from a source (decoded frame).
    Source { frame_id: u64 },
    /// Apply an effect to the input.
    Effect { effect_name: String },
    /// Composite two layers using a blend mode.
    Composite { blend_mode: u32, opacity: f32 },
    /// Transform (translate/rotate/scale) the input.
    Transform { matrix: [f32; 9] },
    /// Final output target.
    Output,
}

/// A node in the render graph.
#[derive(Debug, Clone)]
pub struct RenderNode {
    pub id: NodeId,
    pub op: NodeOp,
    /// Input node IDs (data dependencies).
    pub inputs: Vec<NodeId>,
    /// Width and height of the output texture.
    pub output_size: (u32, u32),
}

/// The complete render graph for one frame.
#[derive(Debug, Default)]
pub struct RenderGraph {
    nodes: HashMap<NodeId, RenderNode>,
    next_id: u32,
}

impl RenderGraph {
    /// Create a new empty render graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node and return its ID.
    pub fn add_node(&mut self, op: NodeOp, inputs: Vec<NodeId>, output_size: (u32, u32)) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        self.nodes.insert(
            id,
            RenderNode {
                id,
                op,
                inputs,
                output_size,
            },
        );
        id
    }

    /// Get a node by ID.
    pub fn node(&self, id: NodeId) -> Option<&RenderNode> {
        self.nodes.get(&id)
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Perform topological sort (Kahn's algorithm).
    /// Returns nodes in execution order, or None if there's a cycle.
    pub fn topological_sort(&self) -> Option<Vec<NodeId>> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        let mut dependents: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        // Initialize in-degrees
        for (&id, node) in &self.nodes {
            in_degree.entry(id).or_insert(0);
            for &input in &node.inputs {
                *in_degree.entry(id).or_insert(0) += 1;
                dependents.entry(input).or_default().push(id);
            }
        }

        // Start with nodes that have no inputs
        let mut queue: Vec<NodeId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        queue.sort_by_key(|id| id.0); // deterministic order

        let mut result = Vec::with_capacity(self.nodes.len());

        while let Some(id) = queue.pop() {
            result.push(id);
            if let Some(deps) = dependents.get(&id) {
                for &dep in deps {
                    let deg = in_degree.get_mut(&dep).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(dep);
                        queue.sort_by_key(|id| id.0);
                    }
                }
            }
        }

        if result.len() == self.nodes.len() {
            Some(result)
        } else {
            None // Cycle detected
        }
    }

    /// Clear the graph for reuse.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.next_id = 0;
    }
}

// ── Frame cache ─────────────────────────────────────────────────

/// Multi-level frame cache (CPU RAM only; GPU cache is managed by TexturePool).
pub struct FrameCache {
    /// Cached decoded frames (frame_id → frame data).
    entries: HashMap<u64, CacheEntry>,
    /// Total memory used.
    memory_used: usize,
    /// Maximum memory budget.
    max_memory: usize,
    /// LRU order (most recently used last).
    lru_order: Vec<u64>,
}

struct CacheEntry {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl FrameCache {
    /// Create a new frame cache with the given memory budget.
    pub fn new(max_memory: usize) -> Self {
        Self {
            entries: HashMap::new(),
            memory_used: 0,
            max_memory,
            lru_order: Vec::new(),
        }
    }

    /// Check if a frame is in the cache.
    pub fn contains(&self, frame_id: u64) -> bool {
        self.entries.contains_key(&frame_id)
    }

    /// Get cached frame data.
    pub fn get(&mut self, frame_id: u64) -> Option<(&[u8], u32, u32)> {
        if self.entries.contains_key(&frame_id) {
            // Move to end of LRU
            self.lru_order.retain(|&id| id != frame_id);
            self.lru_order.push(frame_id);
            let entry = self.entries.get(&frame_id).unwrap();
            Some((&entry.data, entry.width, entry.height))
        } else {
            None
        }
    }

    /// Insert a frame into the cache. Evicts old frames if necessary.
    pub fn insert(&mut self, frame_id: u64, data: Vec<u8>, width: u32, height: u32) {
        let size = data.len();

        // Evict until we have room
        while self.memory_used + size > self.max_memory && !self.lru_order.is_empty() {
            let oldest = self.lru_order.remove(0);
            if let Some(entry) = self.entries.remove(&oldest) {
                self.memory_used -= entry.data.len();
            }
        }

        self.memory_used += size;
        self.lru_order.push(frame_id);
        self.entries.insert(
            frame_id,
            CacheEntry {
                data,
                width,
                height,
            },
        );
    }

    /// Number of cached frames.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Current memory usage.
    pub fn memory_usage(&self) -> usize {
        self.memory_used
    }

    /// Clear the entire cache.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.lru_order.clear();
        self.memory_used = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_sort_linear() {
        let mut graph = RenderGraph::new();
        let a = graph.add_node(NodeOp::Source { frame_id: 0 }, vec![], (1920, 1080));
        let b = graph.add_node(
            NodeOp::Effect {
                effect_name: "blur".into(),
            },
            vec![a],
            (1920, 1080),
        );
        let c = graph.add_node(NodeOp::Output, vec![b], (1920, 1080));

        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted.len(), 3);
        // a must come before b, b before c
        let pos_a = sorted.iter().position(|&id| id == a).unwrap();
        let pos_b = sorted.iter().position(|&id| id == b).unwrap();
        let pos_c = sorted.iter().position(|&id| id == c).unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_topological_sort_diamond() {
        let mut graph = RenderGraph::new();
        let src = graph.add_node(NodeOp::Source { frame_id: 0 }, vec![], (1920, 1080));
        let left = graph.add_node(
            NodeOp::Effect {
                effect_name: "blur".into(),
            },
            vec![src],
            (1920, 1080),
        );
        let right = graph.add_node(
            NodeOp::Effect {
                effect_name: "sharpen".into(),
            },
            vec![src],
            (1920, 1080),
        );
        let merge = graph.add_node(
            NodeOp::Composite {
                blend_mode: 0,
                opacity: 1.0,
            },
            vec![left, right],
            (1920, 1080),
        );

        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted.len(), 4);
        let pos_src = sorted.iter().position(|&id| id == src).unwrap();
        let pos_merge = sorted.iter().position(|&id| id == merge).unwrap();
        assert!(pos_src < pos_merge);
    }

    #[test]
    fn test_topological_sort_cycle() {
        let mut graph = RenderGraph::new();
        // Manually create a cycle by constructing nodes that reference each other.
        // We can't directly create a cycle with add_node since IDs are sequential,
        // but we can test with a self-referencing node.
        let a = graph.add_node(NodeOp::Source { frame_id: 0 }, vec![], (100, 100));
        let _b = graph.add_node(
            NodeOp::Effect {
                effect_name: "test".into(),
            },
            vec![a, NodeId(99)], // reference non-existent node
            (100, 100),
        );
        // Node _b depends on NodeId(99) which doesn't exist, so its in-degree
        // will never reach 0 → sort fails.
        let sorted = graph.topological_sort();
        assert!(sorted.is_none());
    }

    #[test]
    fn test_frame_cache_basic() {
        let mut cache = FrameCache::new(1024);
        cache.insert(0, vec![0u8; 100], 10, 10);
        assert!(cache.contains(0));
        assert_eq!(cache.len(), 1);

        let (data, w, h) = cache.get(0).unwrap();
        assert_eq!(data.len(), 100);
        assert_eq!(w, 10);
        assert_eq!(h, 10);
    }

    #[test]
    fn test_frame_cache_eviction() {
        let mut cache = FrameCache::new(200);
        cache.insert(0, vec![0u8; 100], 10, 10);
        cache.insert(1, vec![0u8; 100], 10, 10);
        assert_eq!(cache.len(), 2);

        // This should evict frame 0 (LRU)
        cache.insert(2, vec![0u8; 100], 10, 10);
        assert_eq!(cache.len(), 2);
        assert!(!cache.contains(0));
        assert!(cache.contains(1));
        assert!(cache.contains(2));
    }

    #[test]
    fn test_frame_cache_lru_update() {
        let mut cache = FrameCache::new(200);
        cache.insert(0, vec![0u8; 100], 10, 10);
        cache.insert(1, vec![0u8; 100], 10, 10);

        // Access frame 0, making frame 1 the LRU
        cache.get(0);

        // Insert frame 2 — should evict frame 1 (now LRU)
        cache.insert(2, vec![0u8; 100], 10, 10);
        assert!(cache.contains(0));
        assert!(!cache.contains(1));
        assert!(cache.contains(2));
    }
}
