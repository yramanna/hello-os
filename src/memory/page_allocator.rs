//! Physical page allocator with 4KB and 2MB page support

use super::multiboot2::MemoryMapTag;
use super::mutex::Mutex;

const PAGE_SIZE_4KB: usize = 4096;
const PAGE_SIZE_2MB: usize = 2 * 1024 * 1024;
const PAGES_PER_2MB: usize = 512;

/// Page size enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSize {
    Size4KB,
    Size2MB,
}

/// Page state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageState {
    Unavailable,      // Reserved, kernel, ACPI, etc.
    Free4KB,          // Free 4KB page
    Free2MB,          // Free 2MB page
    Allocated4KB,     // Allocated 4KB page
    Allocated2MB,     // Allocated 2MB page (head of superpage)
}

/// Metadata for a single page
#[derive(Debug, Clone, Copy)]
struct PageMetadata {
    state: PageState,
    next: Option<usize>,  // Index in page_array for linked list
    prev: Option<usize>,  // Index in page_array for linked list
    counter: u16,         // For 2MB pages: count of free 4KB pages within
}

impl PageMetadata {
    const fn new() -> Self {
        Self {
            state: PageState::Unavailable,
            next: None,
            prev: None,
            counter: 0,
        }
    }
}

/// The physical page allocator
pub struct PageAllocator {
    page_array: Option<&'static mut [PageMetadata]>,
    free_4kb_list: Mutex<Option<usize>>,  // Head index of 4KB free list
    free_2mb_list: Mutex<Option<usize>>,  // Head index of 2MB free list
    kernel_end: usize,
    total_pages: usize,
}

impl PageAllocator {
    pub const fn new() -> Self {
        Self {
            page_array: None,
            free_4kb_list: Mutex::new(None),
            free_2mb_list: Mutex::new(None),
            kernel_end: 0,
            total_pages: 0,
        }
    }

    /// Get a mutable reference to the page array
    unsafe fn get_page_array_mut(&self) -> &mut [PageMetadata] {
        let ptr = self.page_array.as_ref().unwrap().as_ptr() as *mut PageMetadata;
        let len = self.page_array.as_ref().unwrap().len();
        core::slice::from_raw_parts_mut(ptr, len)
    }

    /// Initialize the page allocator
    /// 
    /// # Safety
    /// Must be called exactly once during kernel initialization
    pub unsafe fn init(&mut self, max_physical_addr: u64, mmap: &MemoryMapTag) {
        use crate::println;
        
        // Find the actual highest usable memory from the memory map
        let mut actual_max_addr = 0u64;
        for entry in mmap.memory_areas() {
            if entry.typ == 1 {  // Only consider available RAM
                let end_addr = entry.base_addr + entry.length;
                if end_addr > actual_max_addr {
                    actual_max_addr = end_addr;
                }
            }
        }
        
        // Cap at 4GB to avoid excessive metadata (adjust as needed)
        let capped_max = actual_max_addr.min(4 * 1024 * 1024 * 1024); // 4GB max
        
        // Calculate number of pages needed
        let total_pages = ((capped_max as usize) + PAGE_SIZE_4KB - 1) / PAGE_SIZE_4KB;
        self.total_pages = total_pages;
        
        // Get the kernel end address
        extern "C" {
            static __end: u8;
        }
        let kernel_end_raw = &__end as *const u8 as usize;
        
        // Round up to next page boundary
        let kernel_end = (kernel_end_raw + PAGE_SIZE_4KB - 1) & !(PAGE_SIZE_4KB - 1);
        
        // Calculate size needed for page_array
        let metadata_size = total_pages * core::mem::size_of::<PageMetadata>();
        let metadata_pages = (metadata_size + PAGE_SIZE_4KB - 1) / PAGE_SIZE_4KB;
        
        // Allocate page_array right after kernel_end
        let page_array_addr = kernel_end;
        let page_array_ptr = page_array_addr as *mut PageMetadata;
        let page_array_slice = core::slice::from_raw_parts_mut(page_array_ptr, total_pages);
        
        // Initialize all pages as Unavailable
        for i in 0..total_pages {
            page_array_slice[i] = PageMetadata::new();
        }
        
        self.page_array = Some(page_array_slice);
        self.kernel_end = page_array_addr + metadata_size;
        
        // Round kernel_end up to next page
        self.kernel_end = (self.kernel_end + PAGE_SIZE_4KB - 1) & !(PAGE_SIZE_4KB - 1);
        
        // Mark available memory regions
        for entry in mmap.memory_areas() {
            if entry.typ == 1 {  // Available RAM
                self.mark_available_region(entry.base_addr as usize, entry.length as usize);
            }
        }
        
        // Build free lists
        self.build_free_lists();
        
    }

    /// Mark a memory region as available
    fn mark_available_region(&mut self, base: usize, length: usize) {
        let page_array = self.page_array.as_mut().unwrap();
        
        let start_pfn = base / PAGE_SIZE_4KB;
        let end_pfn = (base + length) / PAGE_SIZE_4KB;
        
        let kernel_end_pfn = self.kernel_end / PAGE_SIZE_4KB;
        
        for pfn in start_pfn..end_pfn {
            if pfn >= page_array.len() {
                break;
            }
            
            // Don't mark kernel pages as available
            if pfn < kernel_end_pfn {
                continue;
            }
            
            // Check if this is aligned to 2MB boundary
            let phys_addr = pfn * PAGE_SIZE_4KB;
            if phys_addr % PAGE_SIZE_2MB == 0 && pfn + PAGES_PER_2MB <= end_pfn {
                // This can be a 2MB page
                page_array[pfn].state = PageState::Free2MB;
            } else if phys_addr >= self.kernel_end {
                // Mark as 4KB page
                page_array[pfn].state = PageState::Free4KB;
            }
        }
    }

    /// Build the free page lists
    fn build_free_lists(&mut self) {
        let page_array = self.page_array.as_mut().unwrap();
        
        let mut free_4kb_head: Option<usize> = None;
        let mut free_2mb_head: Option<usize> = None;
        
        for pfn in 0..page_array.len() {
            match page_array[pfn].state {
                PageState::Free4KB => {
                    // Add to 4KB list
                    page_array[pfn].next = free_4kb_head;
                    page_array[pfn].prev = None;
                    
                    if let Some(old_head) = free_4kb_head {
                        page_array[old_head].prev = Some(pfn);
                    }
                    
                    free_4kb_head = Some(pfn);
                }
                PageState::Free2MB => {
                    // Add to 2MB list and initialize counter
                    page_array[pfn].counter = PAGES_PER_2MB as u16;
                    page_array[pfn].next = free_2mb_head;
                    page_array[pfn].prev = None;
                    
                    if let Some(old_head) = free_2mb_head {
                        page_array[old_head].prev = Some(pfn);
                    }
                    
                    free_2mb_head = Some(pfn);
                }
                _ => {}
            }
        }
        
        *self.free_4kb_list.lock() = free_4kb_head;
        *self.free_2mb_list.lock() = free_2mb_head;
    }

    /// Allocate a page
    pub fn allocate_page(&self, size: PageSize) -> Option<usize> {
        match size {
            PageSize::Size4KB => self.allocate_4kb(),
            PageSize::Size2MB => self.allocate_2mb(),
        }
    }

    /// Allocate a 4KB page
    fn allocate_4kb(&self) -> Option<usize> {
        let mut list_head = self.free_4kb_list.lock();
        
        if let Some(pfn) = *list_head {
            let page_array = unsafe { self.get_page_array_mut() };
            
            // Remove from free list
            let next = page_array[pfn].next;
            *list_head = next;
            
            if let Some(next_pfn) = next {
                page_array[next_pfn].prev = None;
            }
            
            // Mark as allocated
            page_array[pfn].state = PageState::Allocated4KB;
            page_array[pfn].next = None;
            page_array[pfn].prev = None;
            
            // Update counter in superpage head
            drop(list_head);
            self.update_superpage_counter(pfn, -1);
            
            Some(pfn * PAGE_SIZE_4KB)
        } else {
            // No 4KB pages available, try splitting a 2MB page
            drop(list_head);
            self.split_2mb_page()?;
            self.allocate_4kb()
        }
    }

    /// Allocate a 2MB page
    fn allocate_2mb(&self) -> Option<usize> {
        let mut list_head = self.free_2mb_list.lock();
        
        if let Some(pfn) = *list_head {
            let page_array = unsafe { self.get_page_array_mut() };
            
            // Remove from free list
            let next = page_array[pfn].next;
            *list_head = next;
            
            if let Some(next_pfn) = next {
                page_array[next_pfn].prev = None;
            }
            
            // Mark as allocated
            page_array[pfn].state = PageState::Allocated2MB;
            page_array[pfn].next = None;
            page_array[pfn].prev = None;
            
            Some(pfn * PAGE_SIZE_4KB)
        } else {
            None
        }
    }

    /// Split a 2MB page into 512 4KB pages
    fn split_2mb_page(&self) -> Option<()> {
        let mut list_2mb = self.free_2mb_list.lock();
        let pfn_2mb = (*list_2mb)?;
        
        let page_array = unsafe { self.get_page_array_mut() };
        
        // Remove from 2MB list
        let next = page_array[pfn_2mb].next;
        *list_2mb = next;
        
        if let Some(next_pfn) = next {
            page_array[next_pfn].prev = None;
        }
        
        drop(list_2mb);
        
        // Mark all 512 pages as Free4KB and add to 4KB list
        let mut list_4kb = self.free_4kb_list.lock();
        
        for i in 0..PAGES_PER_2MB {
            let pfn = pfn_2mb + i;
            page_array[pfn].state = PageState::Free4KB;
            page_array[pfn].next = *list_4kb;
            page_array[pfn].prev = None;
            
            if let Some(old_head) = *list_4kb {
                page_array[old_head].prev = Some(pfn);
            }
            
            *list_4kb = Some(pfn);
        }
        
        // Set counter in head page
        page_array[pfn_2mb].counter = PAGES_PER_2MB as u16;
        
        Some(())
    }

    /// Free a page
    pub fn free_page(&self, addr: usize, size: PageSize) {
        let pfn = addr / PAGE_SIZE_4KB;
        
        match size {
            PageSize::Size4KB => self.free_4kb(pfn),
            PageSize::Size2MB => self.free_2mb(pfn),
        }
    }

    /// Free a 4KB page
    fn free_4kb(&self, pfn: usize) {
        let page_array = unsafe { self.get_page_array_mut() };
        
        // Update counter
        let can_merge = self.update_superpage_counter(pfn, 1);
        
        // Mark as free
        page_array[pfn].state = PageState::Free4KB;
        
        // Add to free list
        let mut list_head = self.free_4kb_list.lock();
        page_array[pfn].next = *list_head;
        page_array[pfn].prev = None;
        
        if let Some(old_head) = *list_head {
            page_array[old_head].prev = Some(pfn);
        }
        
        *list_head = Some(pfn);
        
        drop(list_head);
        
        // Try to merge if all pages in superpage are free
        if can_merge {
            self.try_merge_superpage(pfn);
        }
    }

    /// Free a 2MB page
    fn free_2mb(&self, pfn: usize) {
        let page_array = unsafe { self.get_page_array_mut() };
        
        page_array[pfn].state = PageState::Free2MB;
        page_array[pfn].counter = PAGES_PER_2MB as u16;
        
        let mut list_head = self.free_2mb_list.lock();
        page_array[pfn].next = *list_head;
        page_array[pfn].prev = None;
        
        if let Some(old_head) = *list_head {
            page_array[old_head].prev = Some(pfn);
        }
        
        *list_head = Some(pfn);
    }

    /// Update the counter in the superpage head
    /// Returns true if the superpage is now fully free
    fn update_superpage_counter(&self, pfn: usize, delta: i32) -> bool {
        let superpage_head = (pfn / PAGES_PER_2MB) * PAGES_PER_2MB;
        let page_array = unsafe { self.get_page_array_mut() };
        
        let new_counter = (page_array[superpage_head].counter as i32 + delta) as u16;
        page_array[superpage_head].counter = new_counter;
        
        new_counter == PAGES_PER_2MB as u16
    }

    /// Try to merge a superpage back to 2MB
    fn try_merge_superpage(&self, pfn: usize) {
        let superpage_head = (pfn / PAGES_PER_2MB) * PAGES_PER_2MB;
        let page_array = unsafe { self.get_page_array_mut() };
        
        // Check if all pages are free
        for i in 0..PAGES_PER_2MB {
            if page_array[superpage_head + i].state != PageState::Free4KB {
                return;
            }
        }
        
        // Remove all 4KB pages from the free list
        for i in 0..PAGES_PER_2MB {
            let pfn = superpage_head + i;
            self.remove_from_4kb_list(pfn);
        }
        
        // Add as 2MB page
        page_array[superpage_head].state = PageState::Free2MB;
        page_array[superpage_head].counter = PAGES_PER_2MB as u16;
        
        let mut list_head = self.free_2mb_list.lock();
        page_array[superpage_head].next = *list_head;
        page_array[superpage_head].prev = None;
        
        if let Some(old_head) = *list_head {
            page_array[old_head].prev = Some(superpage_head);
        }
        
        *list_head = Some(superpage_head);
    }

    /// Remove a page from the 4KB free list
    fn remove_from_4kb_list(&self, pfn: usize) {
        let page_array = unsafe { self.get_page_array_mut() };
        
        let prev = page_array[pfn].prev;
        let next = page_array[pfn].next;
        
        if let Some(prev_pfn) = prev {
            page_array[prev_pfn].next = next;
        } else {
            // This is the head
            let mut list_head = self.free_4kb_list.lock();
            *list_head = next;
        }
        
        if let Some(next_pfn) = next {
            page_array[next_pfn].prev = prev;
        }
        
        page_array[pfn].next = None;
        page_array[pfn].prev = None;
    }
}
