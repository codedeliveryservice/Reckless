#[cfg(feature = "numa")]
mod api {
    use libc::{c_int, c_void, size_t};

    #[link(name = "numa")]
    extern "C" {
        pub fn numa_available() -> c_int;
        pub fn numa_alloc_onnode(size: size_t, node: c_int) -> *mut c_void;
        pub fn numa_free(mem: *mut c_void, size: size_t);
        pub fn numa_max_node() -> c_int;
        pub fn numa_node_of_cpu(cpu: c_int) -> c_int;
        pub fn numa_num_configured_cpus() -> c_int;
        pub fn numa_bitmask_alloc(ncpus: c_uint) -> *mut Bitmask;
        pub fn numa_bitmask_free(bmp: *mut Bitmask);
        pub fn numa_node_to_cpus(node: c_int, buffer: *mut Bitmask) -> c_int;
    }

    #[repr(C)]
    pub struct Bitmask {
        pub size: libc::c_ulong,
        pub maskp: *mut libc::c_ulong,
    }

    extern "C" {
        pub fn sched_getcpu() -> c_int;
    }

    use libc::c_uint;
}

#[cfg(feature = "numa")]
fn get_nodes_with_cpus() -> Vec<libc::c_int> {
    let mut nodes_with_cpus = Vec::new();
    let max_nodes = unsafe { api::numa_max_node() } as usize + 1;
    let num_cpus = unsafe { api::numa_num_configured_cpus() };

    if num_cpus <= 0 {
        println!("NUMA: No configured CPUs found.");
        return nodes_with_cpus;
    }

    let bitmask = unsafe { api::numa_bitmask_alloc(num_cpus as libc::c_uint) };
    if bitmask.is_null() {
        println!("NUMA: Failed to allocate bitmask.");
        return nodes_with_cpus;
    }

    for node in 0..max_nodes {
        let result = unsafe { api::numa_node_to_cpus(node as libc::c_int, bitmask) };
        if result == 0 {
            if unsafe { has_any_cpus_set(bitmask, num_cpus) } {
                println!("NUMA: Node {} has CPUs assigned.", node);
                nodes_with_cpus.push(node as libc::c_int);
            }
        }
    }

    unsafe { api::numa_bitmask_free(bitmask) };
    println!("NUMA: Nodes with CPUs: {:?}", nodes_with_cpus);
    nodes_with_cpus
}

#[cfg(feature = "numa")]
unsafe fn has_any_cpus_set(bitmask: *mut api::Bitmask, num_cpus: libc::c_int) -> bool {
    if bitmask.is_null() || num_cpus <= 0 {
        return false;
    }

    let total_slots =
        (num_cpus as usize + std::mem::size_of::<libc::c_ulong>() * 8 - 1) / (std::mem::size_of::<libc::c_ulong>() * 8);

    for i in 0..total_slots {
        if *(&*bitmask).maskp.add(i) != 0 {
            return true;
        }
    }

    false
}

pub unsafe trait NumaValue: Sync {}

#[allow(dead_code)]
pub struct NumaReplicator<T: NumaValue> {
    nodes: Vec<*mut T>,
    node_mapping: Vec<libc::c_int>,
    size: Option<usize>,
}

unsafe impl<T: NumaValue> Send for NumaReplicator<T> {}
unsafe impl<T: NumaValue> Sync for NumaReplicator<T> {}

impl<T: NumaValue> NumaReplicator<T> {
    pub fn new(source: &'static T) -> Self {
        #[cfg(feature = "numa")]
        {
            if unsafe { api::numa_available() } < 0 {
                return Self::fallback(source);
            }

            let size = std::mem::size_of::<T>();
            let cpu_nodes = get_nodes_with_cpus();

            if cpu_nodes.is_empty() {
                return Self::fallback(source);
            }

            let nodes = cpu_nodes
                .iter()
                .map(|&node| {
                    let ptr = unsafe { api::numa_alloc_onnode(size, node) } as *mut T;
                    println!("NUMA: Allocated memory on node {} at {:p}", node, ptr);

                    if ptr.is_null() {
                        panic!("Failed to allocate NUMA memory on node {node}");
                    }

                    // SAFETY: T: NumaValue guarantees a byte-copy is valid for read-only sharing.
                    unsafe { std::ptr::copy_nonoverlapping(source, ptr, 1) };
                    ptr
                })
                .collect::<Vec<_>>();

            println!("NUMA: Created NumaReplicator with {} nodes.", nodes.len());

            Self { nodes, node_mapping: cpu_nodes, size: Some(size) }
        }

        #[cfg(not(feature = "numa"))]
        Self::fallback(source)
    }

    pub fn get_local_copy(&self) -> &T {
        unsafe { &*(self.nodes[self.current_node()]) }
    }

    fn current_node(&self) -> usize {
        #[cfg(feature = "numa")]
        {
            let actual_node = unsafe { api::numa_node_of_cpu(api::sched_getcpu()) };
            for (index, &mapped_node) in self.node_mapping.iter().enumerate() {
                if mapped_node == actual_node {
                    return index;
                }
            }
        }

        0
    }

    fn fallback(source: &'static T) -> Self {
        let ptr = source as *const T as *mut T;
        Self { nodes: vec![ptr], node_mapping: vec![0], size: None }
    }
}

#[cfg(feature = "numa")]
impl<T: NumaValue> Drop for NumaReplicator<T> {
    fn drop(&mut self) {
        if let Some(size) = self.size {
            for &ptr in &self.nodes {
                unsafe { api::numa_free(ptr.cast::<libc::c_void>(), size) };
            }
        }
    }
}
