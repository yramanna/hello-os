//! Memory allocator with 4KB and 2MB page support

pub mod multiboot2;
pub mod page_allocator;
pub mod mutex;
pub mod test;

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{self, NonNull};
use core::mem;

use page_allocator::{PageAllocator, PageSize};
use mutex::Mutex;

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
    
    // Initialize the heap allocator
    ALLOCATOR.init();
}

/// Get a reference to the global page allocator
pub fn get_allocator() -> &'static PageAllocator {
    unsafe { &PAGE_ALLOCATOR }
}

/// Node in the free list
struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

/// A simple heap allocator using a linked list of free blocks
pub struct HeapAllocator {
    head: Mutex<Option<&'static mut ListNode>>,
}

impl HeapAllocator {
    pub const fn new() -> Self {
        HeapAllocator {
            head: Mutex::new(None),
        }
    }

    /// Initialize the heap allocator by allocating initial pages
    pub unsafe fn init(&self) {
        // Allocate some initial heap space (e.g., 16 pages = 64KB)
        const INITIAL_HEAP_PAGES: usize = 16;
        
        for _ in 0..INITIAL_HEAP_PAGES {
            if let Some(page_addr) = PAGE_ALLOCATOR.allocate_page(PageSize::Size4KB) {
                self.add_free_region(page_addr, 4096);
            }
        }
    }

    /// Add a free region to the heap
    unsafe fn add_free_region(&self, addr: usize, size: usize) {
        // Ensure the region is large enough to hold a ListNode
        assert!(size >= mem::size_of::<ListNode>());
        assert!(addr % mem::align_of::<ListNode>() == 0);

        let mut node = ListNode::new(size);
        node.next = self.head.lock().take();
        
        let node_ptr = addr as *mut ListNode;
        node_ptr.write(node);
        
        *self.head.lock() = Some(&mut *node_ptr);
    }

    /// Find a free region that can fit the given size and alignment
    fn find_region(&self, size: usize, align: usize) -> Option<(usize, usize)> {
        let mut head = self.head.lock();
        let mut current = head.as_mut()?;
        
        // Check if head works
        if let Some(alloc_start) = Self::alloc_from_region(&*current, size, align) {
            let next = current.next.take();
            let region_start = current.start_addr();
            let region_end = current.end_addr();
            *head = next;
            return Some((alloc_start, region_end));
        }

        // Check rest of list
        loop {
            let next = match current.next.as_mut() {
                Some(next) => next,
                None => return None,
            };

            if let Some(alloc_start) = Self::alloc_from_region(&*next, size, align) {
                let region_end = next.end_addr();
                current.next = next.next.take();
                return Some((alloc_start, region_end));
            }

            current = current.next.as_mut()?;
        }
    }

    /// Try to allocate from a region
    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Option<usize> {
        let alloc_start = Self::align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size)?;

        if alloc_end > region.end_addr() {
            return None;
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            return None;
        }

        Some(alloc_start)
    }

    fn align_up(addr: usize, align: usize) -> usize {
        (addr + align - 1) & !(align - 1)
    }

    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for HeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = HeapAllocator::size_align(layout);
        
        if let Some((alloc_start, region_end)) = self.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess_size = region_end - alloc_end;
            
            if excess_size > 0 {
                self.add_free_region(alloc_end, excess_size);
            }
            
            alloc_start as *mut u8
        } else {
            // Try to allocate more pages from the page allocator
            let pages_needed = (size + 4095) / 4096;
            
            // Allocate contiguous pages
            if pages_needed <= 8 {  // Reasonable limit
                // Allocate multiple pages and add them as one contiguous region
                if let Some(first_page) = PAGE_ALLOCATOR.allocate_page(PageSize::Size4KB) {
                    let mut all_contiguous = true;
                    let mut last_page = first_page;
                    
                    // Try to allocate remaining pages
                    for i in 1..pages_needed {
                        if let Some(page) = PAGE_ALLOCATOR.allocate_page(PageSize::Size4KB) {
                            // Check if pages are contiguous
                            if page != last_page + 4096 {
                                all_contiguous = false;
                                // Free the non-contiguous page
                                PAGE_ALLOCATOR.free_page(page, PageSize::Size4KB);
                                break;
                            }
                            last_page = page;
                        } else {
                            all_contiguous = false;
                            break;
                        }
                    }
                    
                    if all_contiguous && last_page == first_page + (pages_needed - 1) * 4096 {
                        // All pages are contiguous, add as one large region
                        self.add_free_region(first_page, pages_needed * 4096);
                        return self.alloc(layout);
                    } else {
                        // Not contiguous, free what we allocated and add individually
                        let mut addr = first_page;
                        while addr <= last_page {
                            self.add_free_region(addr, 4096);
                            addr += 4096;
                        }
                        return self.alloc(layout);
                    }
                }
            }
            
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = HeapAllocator::size_align(layout);
        self.add_free_region(ptr as usize, size);
    }
}

#[global_allocator]
static ALLOCATOR: HeapAllocator = HeapAllocator::new();
