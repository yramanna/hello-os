//! Memory allocator with 4KB and 2MB page support

pub mod multiboot2;
pub mod page_allocator;
pub mod mutex;
pub mod test;

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

use page_allocator::{PageAllocator, PageSize};

/// The global page allocator instance
static mut PAGE_ALLOCATOR: PageAllocator = PageAllocator::new();

/// Initialize the memory subsystem
/// 
/// # Safety
/// Must be called exactly once during kernel initialization
pub unsafe fn init(multiboot_info_addr: usize) {
    use crate::println;
        
    // Parse multiboot information
    let boot_info = multiboot2::BootInfo::parse(multiboot_info_addr as *const u8)
        .expect("Failed to parse multiboot info");
    
    // Find the memory map tag
    let mmap_tag = boot_info.memory_map_tag()
        .expect("No memory map found in multiboot info");
    
    // Find maximum physical address
    let mut max_addr = 0u64;
    for entry in mmap_tag.memory_areas() {
        let end_addr = entry.base_addr + entry.length;
        if end_addr > max_addr {
            max_addr = end_addr;
        }
    }
        
    // Initialize the page allocator
    PAGE_ALLOCATOR.init(max_addr, mmap_tag);
    
}

/// Get a reference to the global page allocator
pub fn get_allocator() -> &'static PageAllocator {
    unsafe { &PAGE_ALLOCATOR }
}

/// Global allocator for Box<T> and other heap allocations
pub struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // For simplicity, always allocate a 4KB page regardless of size
        // In production, you'd want to handle small allocations differently
        if layout.size() > 4096 {
            // For large allocations, we could use 2MB pages
            // but for now just fail
            return null_mut();
        }
        
        match PAGE_ALLOCATOR.allocate_page(PageSize::Size4KB) {
            Some(page_addr) => page_addr as *mut u8,
            None => null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        PAGE_ALLOCATOR.free_page(ptr as usize, PageSize::Size4KB);
    }
}

#[global_allocator]
static ALLOCATOR: KernelAllocator = KernelAllocator;
