//! Memory allocator with 4KB and 2MB page support

pub mod multiboot2;
pub mod page_allocator;
pub mod mutex;
pub mod test;

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

use page_allocator::{PageAllocator, PageSize};

/// The global page allocator instance
static PAGE_ALLOCATOR: PageAllocator = PageAllocator::new();

/// Initialize the memory subsystem
/// 
/// # Safety
/// Must be called exactly once during kernel initialization
pub unsafe fn init(multiboot_info_addr: usize) {
    use crate::println;
    
    println!("Initializing memory allocator...");
    println!("Multiboot info at: {:#x}", multiboot_info_addr);
    
    // Parse multiboot information
    let boot_info = multiboot2::BootInfo::parse(multiboot_info_addr as *const u8)
        .expect("Failed to parse multiboot info");
    
    println!("Boot info parsed successfully");
    
    // Find the memory map tag
    let mmap_tag = boot_info.memory_map_tag()
        .expect("No memory map found in multiboot info");
    
    println!("Memory map found");
    
    // Find maximum physical address
    let mut max_addr = 0u64;
    let mut total_available = 0u64;
    for entry in mmap_tag.memory_areas() {
        println!("Memory region: base={:#x}, len={:#x}, type={}", 
                 entry.base_addr, entry.length, entry.typ);
        let end_addr = entry.base_addr + entry.length;
        if end_addr > max_addr {
            max_addr = end_addr;
        }
        if entry.typ == 1 {
            total_available += entry.length;
        }
    }
    
    println!("Max physical address: {:#x}", max_addr);
    println!("Total available memory: {} MB", total_available / (1024 * 1024));
    
    // Initialize the page allocator
    PAGE_ALLOCATOR.init(max_addr, mmap_tag);
    
    println!("Page allocator initialized");
}

/// Get a reference to the global page allocator
pub fn get_allocator() -> &'static PageAllocator {
    &PAGE_ALLOCATOR
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
        // For allocations larger than 4KB but up to 2MB
        else if layout.size() <= 2 * 1024 * 1024 {
            // For simplicity, just allocate a 2MB page if we need multiple 4KB pages
            // This wastes memory but avoids complexity of tracking contiguous allocation
            match PAGE_ALLOCATOR.allocate_page(PageSize::Size2MB) {
                Some(addr) => addr as *mut u8,
                None => null_mut(),
            }
        }
        // For 2MB+ allocations
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
        
        // Match the allocation strategy
        if layout.size() <= 4096 {
            PAGE_ALLOCATOR.free_page(addr, PageSize::Size4KB);
        } else {
            // We allocated a 2MB page for anything > 4KB
            PAGE_ALLOCATOR.free_page(addr, PageSize::Size2MB);
        }
    }
}

#[global_allocator]
pub static ALLOCATOR: SimpleAllocator = SimpleAllocator;
