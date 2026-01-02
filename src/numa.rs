#[cfg(feature = "numa")]
use std::{collections::HashMap, sync::OnceLock};

#[cfg(feature = "numa")]
static MAPPING: OnceLock<HashMap<usize, Vec<usize>>> = OnceLock::new();

#[cfg(feature = "numa")]
fn mapping() -> HashMap<usize, Vec<usize>> {
    fn initialize() -> HashMap<usize, Vec<usize>> {
        let mut map = HashMap::new();

        let max_node = unsafe { api::numa_max_node() as usize };
        for node in 0..=max_node {
            let mask = unsafe { api::numa_allocate_cpumask() };
            unsafe { api::numa_node_to_cpus(node as i32, mask) };

            let mut cpus = Vec::new();
            for cpu in 0..libc::CPU_SETSIZE as i32 {
                if unsafe { api::numa_bitmask_isbitset(mask, cpu) } != 0 {
                    cpus.push(cpu as usize);
                }
            }

            unsafe { api::numa_bitmask_free(mask) };

            if !cpus.is_empty() {
                map.insert(node, cpus);
            }
        }

        println!("NUMA mapping initialized: {:?}", map);

        map
    }

    MAPPING.get_or_init(|| initialize()).clone()
}

#[cfg(feature = "numa")]
pub fn bind_thread(id: usize) {
    fn num_cpus() -> usize {
        mapping().values().map(|cpus| cpus.len()).sum()
    }

    let id = id % num_cpus();
    let node = mapping().iter().find_map(|(node, cpus)| cpus.contains(&id).then_some(*node)).unwrap_or(0);

    unsafe {
        api::numa_run_on_node(node as i32);
        api::numa_set_preferred(node as i32);
    }
}

pub unsafe trait NumaValue: Sync {}

pub struct NumaReplicator<T: NumaValue> {
    allocated: Vec<*mut T>,
}

unsafe impl<T: NumaValue> Send for NumaReplicator<T> {}
unsafe impl<T: NumaValue> Sync for NumaReplicator<T> {}

impl<T: NumaValue> NumaReplicator<T> {
    #[cfg(feature = "numa")]
    pub unsafe fn new(source: fn() -> T) -> Self {
        if api::numa_available() < 0 {
            panic!("NUMA is not available on this system");
        }

        let mut allocated = Vec::new();
        let mut nodes = Vec::new();

        for (node, cpus) in mapping() {
            if cpus.is_empty() {
                continue;
            }

            let ptr = api::numa_alloc_onnode(std::mem::size_of::<T>(), node as i32);
            if ptr.is_null() {
                panic!("Failed to allocate memory on NUMA node {node}");
            }

            let tptr = ptr as *mut T;
            std::ptr::write(tptr, source());

            allocated.push(tptr);
            nodes.push(node);
        }

        println!("NumaReplicator allocated on nodes: {:?}", nodes);
        println!("NumaReplicator total replicas: {}", allocated.len());

        Self { allocated }
    }

    #[cfg(not(feature = "numa"))]
    pub unsafe fn new(source: fn() -> T) -> Self {
        let ptr = std::alloc::alloc(std::alloc::Layout::new::<T>()) as *mut T;
        if ptr.is_null() {
            panic!("Failed to allocate memory for NumaReplicator");
        }

        std::ptr::write(ptr, source());

        Self { allocated: vec![ptr] }
    }

    #[cfg(feature = "numa")]
    pub unsafe fn get(&self) -> *const T {
        let cpu = libc::sched_getcpu();
        let node = api::numa_node_of_cpu(cpu);

        let index = mapping().iter().enumerate().find_map(|(i, (n, _))| (*n as i32 == node).then_some(i)).unwrap_or(0);
        &*self.allocated[index]
    }

    #[cfg(not(feature = "numa"))]
    pub unsafe fn get(&self) -> &T {
        &*self.allocated[0]
    }

    pub unsafe fn get_all(&self) -> Vec<&T> {
        self.allocated.iter().map(|&ptr| &*ptr).collect()
    }
}

impl<T: NumaValue> Drop for NumaReplicator<T> {
    fn drop(&mut self) {
        for &ptr in &self.allocated {
            unsafe {
                std::ptr::drop_in_place(ptr);

                #[cfg(feature = "numa")]
                api::numa_free(ptr as *mut libc::c_void, std::mem::size_of::<T>());
            }

            println!("NumaReplicator deallocated");
        }
    }
}

#[allow(dead_code)]
#[cfg(feature = "numa")]
mod api {
    use libc::{c_int, c_void, size_t};

    #[repr(C)]
    pub struct Bitmask {
        size: c_int,
        maskp: *mut u32,
    }

    #[link(name = "numa")]
    extern "C" {
        pub fn numa_available() -> c_int;
        pub fn numa_max_node() -> c_int;
        pub fn numa_node_of_cpu(cpu: c_int) -> c_int;

        pub fn numa_alloc_onnode(size: size_t, node: c_int) -> *mut c_void;
        pub fn numa_free(mem: *mut c_void, size: size_t);

        pub fn numa_run_on_node(node: i32) -> i32;
        pub fn numa_set_preferred(node: i32);

        pub fn numa_node_to_cpus(node: c_int, mask: *mut Bitmask) -> c_int;
        pub fn numa_allocate_cpumask() -> *mut Bitmask;
        pub fn numa_bitmask_free(mask: *mut Bitmask);
        pub fn numa_bitmask_isbitset(mask: *const Bitmask, n: c_int) -> c_int;
    }
}
