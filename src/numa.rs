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

pub unsafe trait NumaValue: Sync {}

#[allow(dead_code)]
pub struct NumaReplicator<T: NumaValue> {
    allocated: Vec<*mut T>,
    available_nodes: Vec<libc::c_int>,
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
            let available_nodes = unsafe { get_numa_node_mapping() };

            if available_nodes.is_empty() {
                return Self::fallback(source);
            }

            let allocated = available_nodes
                .iter()
                .map(|&node| {
                    let ptr = unsafe { api::numa_alloc_onnode(size, node) } as *mut T;
                    if ptr.is_null() {
                        panic!("Failed to allocate NUMA memory on node {node}");
                    }

                    unsafe { std::ptr::copy_nonoverlapping(source, ptr, 1) };
                    ptr
                })
                .collect::<Vec<_>>();

            Self { allocated, available_nodes, size: Some(size) }
        }

        #[cfg(not(feature = "numa"))]
        Self::fallback(source)
    }

    pub fn get_local_copy(&self) -> &T {
        unsafe { &*(self.allocated[self.current_node()]) }
    }

    unsafe fn current_node(&self) -> usize {
        #[cfg(feature = "numa")]
        {
            let actual_node = api::numa_node_of_cpu(api::sched_getcpu());
            for (index, &mapped_node) in self.available_nodes.iter().enumerate() {
                if mapped_node == actual_node {
                    return index;
                }
            }
        }
        0
    }

    fn fallback(source: &'static T) -> Self {
        let ptr = source as *const T as *mut T;
        Self { allocated: vec![ptr], available_nodes: vec![0], size: None }
    }
}

#[cfg(feature = "numa")]
impl<T: NumaValue> Drop for NumaReplicator<T> {
    fn drop(&mut self) {
        if let Some(size) = self.size {
            for &ptr in &self.allocated {
                unsafe { api::numa_free(ptr.cast::<libc::c_void>(), size) };
            }
        }
    }
}

#[cfg(feature = "numa")]
unsafe fn get_numa_node_mapping() -> Vec<libc::c_int> {
    let max_nodes = api::numa_max_node() as usize + 1;
    let num_cpus = api::numa_num_configured_cpus();

    if num_cpus <= 0 {
        return Vec::new();
    }

    let bitmask = api::numa_bitmask_alloc(num_cpus as libc::c_uint);
    if bitmask.is_null() {
        return Vec::new();
    }

    let nodes = (0..max_nodes)
        .filter_map(|node| {
            let result = api::numa_node_to_cpus(node as libc::c_int, bitmask);
            if result == 0 && has_any_cpus_set(bitmask, num_cpus) {
                Some(node as libc::c_int)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    api::numa_bitmask_free(bitmask);
    nodes
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
