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

/// Simple global allocator that wastes a full 4KB page per allocation
/// This matches the assignment specification
pub struct SimpleAllocator;

unsafe impl GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // As per assignment: "waste an entire 4KB page on an object that is smaller than a page"
        if layout.size() == 0 {
            return null_mut();
        }
        
        // For allocations up to 4KB, allocate a 4KB page
        if layout.size() <= 4096 {
            match PAGE_ALLOCATOR.allocate_page(PageSize::Size4KB) {
                Some(addr) => addr as *mut u8,
                None => null_mut(),
            }
        } 
        // For allocations larger than 4KB but less than 2MB, allocate multiple 4KB pages
        else if layout.size() <= 2 * 1024 * 1024 {
            // Calculate how many 4KB pages we need
            let pages_needed = (layout.size() + 4095) / 4096;
            
            // Try to allocate first page
            if let Some(first_page) = PAGE_ALLOCATOR.allocate_page(PageSize::Size4KB) {
                let mut contiguous = true;
                let mut last_page = first_page;
                
                // Try to allocate remaining pages contiguously
                for _ in 1..pages_needed {
                    if let Some(page) = PAGE_ALLOCATOR.allocate_page(PageSize::Size4KB) {
                        if page != last_page + 4096 {
                            // Not contiguous, free what we got and fail
                            contiguous = false;
                            PAGE_ALLOCATOR.free_page(page, PageSize::Size4KB);
                            break;
                        }
                        last_page = page;
                    } else {
                        contiguous = false;
                        break;
                    }
                }
                
                if contiguous {
                    first_page as *mut u8
                } else {
                    // Free all allocated pages
                    let mut addr = first_page;
                    while addr <= last_page {
                        PAGE_ALLOCATOR.free_page(addr, PageSize::Size4KB);
                        addr += 4096;
                    }
                    null_mut()
                }
            } else {
                null_mut()
            }
        }
        // For 2MB allocations
        else {
            match PAGE_ALLOCATOR.allocate_page(PageSize::Size2MB) {
                Some(addr) => addr as *mut u8,
                None => null_mut(),
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() == 0 {
            return;
        }
        
        let addr = ptr as usize;
        
        // Determine what size to free based on layout size
        if layout.size() <= 4096 {
            PAGE_ALLOCATOR.free_page(addr, PageSize::Size4KB);
        } else if layout.size() <= 2 * 1024 * 1024 {
            // Free multiple 4KB pages
            let pages_needed = (layout.size() + 4095) / 4096;
            for i in 0..pages_needed {
                PAGE_ALLOCATOR.free_page(addr + i * 4096, PageSize::Size4KB);
            }
        } else {
            PAGE_ALLOCATOR.free_page(addr, PageSize::Size2MB);
        }
    }
}

#[global_allocator]
pub static ALLOCATOR: SimpleAllocator = SimpleAllocator;
