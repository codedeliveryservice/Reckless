#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_imports)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    sync::{
        Arc, LazyLock, Mutex, RwLock,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
};

pub trait NumaReplicable: Send + Sync + 'static {
    fn allocate() -> Arc<Self>;

    fn allocate_shared() -> Option<Arc<Self>> {
        None
    }
}

type CpuIndex = usize;
type NumaIndex = usize;

static SYSTEM_THREADS: LazyLock<CpuIndex> =
    LazyLock::new(|| thread::available_parallelism().map(|x| x.get()).unwrap_or(1).max(1));

#[cfg(all(target_os = "linux", not(target_os = "android")))]
static PROCESSOR_AFFINITY: LazyLock<BTreeSet<CpuIndex>> = LazyLock::new(get_process_affinity);

#[cfg(all(target_os = "linux", not(target_os = "android")))]
fn get_process_affinity() -> BTreeSet<CpuIndex> {
    use libc::{CPU_ISSET, CPU_SETSIZE, CPU_ZERO, cpu_set_t, sched_getaffinity};

    let mut mask: cpu_set_t = unsafe { std::mem::zeroed() };
    unsafe { CPU_ZERO(&mut mask) };

    let status = unsafe { sched_getaffinity(0, std::mem::size_of::<cpu_set_t>(), &mut mask as *mut cpu_set_t) };
    if status != 0 {
        panic!("sched_getaffinity failed");
    }

    (0..(CPU_SETSIZE as usize)).filter(|&cpu| unsafe { CPU_ISSET(cpu, &mask) }).collect::<BTreeSet<CpuIndex>>()
}

#[derive(Copy, Clone, Default)]
pub struct NumaReplicatedAccessToken {
    index: NumaIndex,
}

impl NumaReplicatedAccessToken {
    pub fn new(index: NumaIndex) -> Self {
        Self { index }
    }
}

#[derive(Clone)]
pub struct NumaConfig {
    nodes: Vec<BTreeSet<CpuIndex>>,
    node_by_cpu: BTreeMap<CpuIndex, NumaIndex>,
    highest_cpu_index: CpuIndex,
}

impl Default for NumaConfig {
    fn default() -> Self {
        let mut cfg = Self::empty();
        for cpu in 0..*SYSTEM_THREADS {
            cfg.add_cpu_to_node(0, cpu);
        }
        cfg
    }
}

impl NumaConfig {
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            node_by_cpu: BTreeMap::new(),
            highest_cpu_index: 0,
        }
    }

    pub fn from_system() -> Self {
        // Fallback for unsupported systems.
        #[cfg(not(all(target_os = "linux", not(target_os = "android"))))]
        return Self::default();

        #[cfg(all(target_os = "linux", not(target_os = "android")))]
        {
            let mut cfg = NumaConfig::from_system_numa();
            cfg.remove_empty_numa_nodes();
            cfg
        }
    }

    pub fn num_numa_nodes(&self) -> NumaIndex {
        self.nodes.len()
    }

    pub fn requires_memory_replication(&self) -> bool {
        self.nodes.len() > 1
    }

    pub fn suggests_binding_threads(&self, threads: CpuIndex) -> bool {
        if !self.requires_memory_replication() || threads <= 1 {
            return false;
        }

        let largest_node_size = self.nodes.iter().map(|node| node.len()).max().unwrap_or(0);

        let is_node_sufficient = |node: &BTreeSet<CpuIndex>| {
            const NODE_THRESHOLD: f64 = 0.6;
            (node.len() as f64) / (largest_node_size as f64) > NODE_THRESHOLD
        };

        let sufficient_nodes = self.nodes.iter().filter(|node| is_node_sufficient(node)).count();
        threads > largest_node_size / 2 || threads >= 4 * sufficient_nodes
    }

    pub fn distribute_threads_among_numa_nodes(&self, num_threads: CpuIndex) -> Vec<NumaIndex> {
        if self.nodes.len() == 1 {
            return vec![0; num_threads];
        }

        let mut nodes = Vec::new();
        let mut occupation = vec![0usize; self.nodes.len()];

        for _ in 0..num_threads {
            let mut best_node = 0;
            let mut best_fill = f32::MAX;

            for (node, _) in self.nodes.iter().enumerate() {
                let fill = (occupation[node] + 1) as f32 / self.nodes[node].len() as f32;
                if fill < best_fill {
                    best_node = node;
                    best_fill = fill;
                }
            }

            nodes.push(best_node);
            occupation[best_node] += 1;
        }

        nodes
    }

    pub fn bind_current_thread_to_numa_node(&self, node: NumaIndex) -> NumaReplicatedAccessToken {
        assert!(node < self.nodes.len() && !self.nodes[node].is_empty());

        #[cfg(all(target_os = "linux", not(target_os = "android")))]
        {
            use libc::{CPU_SET, CPU_ZERO, cpu_set_t, sched_setaffinity, sched_yield};

            let mut mask: cpu_set_t = unsafe { std::mem::zeroed() };
            unsafe { CPU_ZERO(&mut mask) };

            for cpu in &self.nodes[node] {
                unsafe { CPU_SET(*cpu, &mut mask) };
            }

            let status = unsafe { sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &mask as *const cpu_set_t) };
            if status != 0 {
                panic!("sched_setaffinity failed");
            }

            unsafe { sched_yield() };
        }

        NumaReplicatedAccessToken::new(node)
    }

    pub fn execute_on_numa_node<F: FnOnce() + Send + 'static>(&self, n: NumaIndex, f: F) {
        let cfg = self.clone();
        let handle = thread::spawn(move || {
            cfg.bind_current_thread_to_numa_node(n);
            f();
        });
        handle.join().unwrap();
    }

    fn add_cpu_to_node(&mut self, node: NumaIndex, cpu: CpuIndex) {
        if self.nodes.len() <= node {
            self.nodes.resize_with(node + 1, BTreeSet::new);
        }

        self.nodes[node].insert(cpu);
        self.node_by_cpu.insert(cpu, node);
        self.highest_cpu_index = self.highest_cpu_index.max(cpu);
    }

    fn remove_empty_numa_nodes(&mut self) {
        self.nodes.retain(|cpus| !cpus.is_empty());

        self.node_by_cpu.clear();
        for (node, cpus) in self.nodes.iter().enumerate() {
            for &cpu in cpus {
                self.node_by_cpu.insert(cpu, node);
            }
        }

        self.highest_cpu_index = self.node_by_cpu.keys().copied().max().unwrap_or(0);
    }

    fn from_system_numa() -> Self {
        let mut cfg = NumaConfig::empty();

        #[cfg(all(target_os = "linux", not(target_os = "android")))]
        {
            let fallback = || {
                let mut cfg = NumaConfig::empty();
                for cpu in 0..*SYSTEM_THREADS {
                    if PROCESSOR_AFFINITY.contains(&cpu) {
                        cfg.add_cpu_to_node(0, cpu);
                    }
                }
                cfg
            };

            let Ok(node_ids) = fs::read_to_string("/sys/devices/system/node/online").map(remove_whitespace) else {
                return fallback();
            };

            if node_ids.is_empty() {
                return fallback();
            }

            for node in parse_cpu_indices(&node_ids) {
                let path = format!("/sys/devices/system/node/node{node}/cpulist");
                let cpu_ids = fs::read_to_string(&path);
                if cpu_ids.is_err() {
                    return fallback();
                }

                let cpu_ids = remove_whitespace(cpu_ids.unwrap());
                for cpu in parse_cpu_indices(&cpu_ids) {
                    if PROCESSOR_AFFINITY.contains(&cpu) {
                        cfg.add_cpu_to_node(node, cpu);
                    }
                }
            }
        }

        cfg
    }
}

fn parse_cpu_indices(cpu_ids: &str) -> Vec<usize> {
    if cpu_ids.is_empty() {
        return Vec::new();
    }

    let mut indices = Vec::new();
    for segment in cpu_ids.split(',').filter(|s| !s.is_empty()) {
        let parts: Vec<_> = segment.split('-').collect();
        match parts.len() {
            1 => indices.push(parts[0].parse::<usize>().unwrap()),
            2 => {
                let first = parts[0].parse::<usize>().unwrap();
                let last = parts[1].parse::<usize>().unwrap();
                for cpu in first..=last {
                    indices.push(cpu);
                }
            }
            _ => {}
        }
    }
    indices
}

fn remove_whitespace(s: String) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

pub trait NumaReplicatedBase: Send + Sync {
    fn on_numa_config_changed(&self);
    fn get_numa_config(&self) -> NumaConfig;
}

pub struct NumaReplicationContext {
    config: RwLock<NumaConfig>,
    thread_count: AtomicUsize,
    tracked: Mutex<Vec<Arc<dyn NumaReplicatedBase>>>,
}

impl NumaReplicationContext {
    pub fn new(cfg: NumaConfig) -> Self {
        Self {
            config: RwLock::new(cfg),
            thread_count: AtomicUsize::new(1),
            tracked: Mutex::new(Vec::new()),
        }
    }

    pub fn attach(&self, obj: Arc<dyn NumaReplicatedBase>) {
        self.tracked.lock().unwrap().push(obj);
    }

    pub fn get_numa_config(&self) -> NumaConfig {
        self.config.read().unwrap().clone()
    }

    pub fn set_thread_count(&self, threads: usize) {
        let previous = self.thread_count.swap(threads, Ordering::Release);
        if previous == threads {
            return;
        }

        let tracked = self.tracked.lock().unwrap().clone();
        for obj in tracked {
            obj.on_numa_config_changed();
        }
    }

    pub fn get_thread_count(&self) -> usize {
        self.thread_count.load(Ordering::Acquire)
    }
}

pub struct NumaReplicated<T: NumaReplicable> {
    ctx: Arc<NumaReplicationContext>,
    instances: RwLock<Vec<Arc<T>>>,
}

impl<T: NumaReplicable> NumaReplicated<T> {
    pub fn new(ctx: Arc<NumaReplicationContext>) -> Arc<Self> {
        let obj = Arc::new(Self { ctx, instances: RwLock::new(Vec::new()) });
        obj.replicate_instances();
        obj.ctx.attach(obj.clone());
        obj
    }

    pub fn get(&self, token: NumaReplicatedAccessToken) -> Arc<T> {
        self.instances.read().unwrap()[token.index].clone()
    }

    pub fn all(&self) -> Vec<Arc<T>> {
        self.instances.read().unwrap().clone()
    }

    fn replicate_instances(&self) {
        let cfg = self.ctx.get_numa_config();
        let mut instances = Vec::<Arc<T>>::new();

        let allocate_on_node = |node| {
            let (tx, rx) = mpsc::channel();
            cfg.execute_on_numa_node(node, move || {
                tx.send(T::allocate()).expect("failed to send NUMA replicated instance");
            });
            rx.recv().expect("failed to receive NUMA replicated instance")
        };

        if cfg.suggests_binding_threads(self.ctx.get_thread_count()) {
            for node in 0..cfg.num_numa_nodes() {
                instances.push(allocate_on_node(node));
            }
        } else if let Some(shared) = T::allocate_shared() {
            instances.push(shared);
        } else {
            instances.push(allocate_on_node(0));
        }

        *self.instances.write().unwrap() = instances;
    }
}

impl<T: NumaReplicable> NumaReplicatedBase for NumaReplicated<T> {
    fn on_numa_config_changed(&self) {
        self.replicate_instances();
    }

    fn get_numa_config(&self) -> NumaConfig {
        self.ctx.get_numa_config()
    }
}
