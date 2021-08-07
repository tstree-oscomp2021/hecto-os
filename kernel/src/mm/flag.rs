use bitflags::*;

bitflags! {
    pub struct MapFlags: u32 {
        const MAP_SHARED            = 0x01;		/* Share changes */
        const MAP_PRIVATE           = 0x02;		/* Changes are private */
        const MAP_SHARED_VALIDATE   = 0x03;	    /* share + validate extension flags */

        /* 0x01 - 0x03 are defined in linux/mman.h */
        const MAP_TYPE              =       0x0f; /* Mask for type of mapping */
        const MAP_FIXED             =       0x10; /* Interpret addr exactly */
        const MAP_ANONYMOUS         =       0x20; /* don't use a file */

        /* 0x0100 - 0x4000 flags are defined in asm-generic/mman.h */
        const MAP_POPULATE          =  0x00_8000; /* populate (prefault) pagetables */
        const MAP_NONBLOCK          =  0x01_0000; /* do not block on IO */
        const MAP_STACK             =  0x02_0000; /* give out an address that is best suited for process/thread stacks */
        const MAP_HUGETLB           =  0x04_0000; /* create a huge page mapping */
        const MAP_SYNC              =  0x08_0000; /* perform synchronous page faults for the mapping */
        const MAP_FIXED_NOREPLACE   =  0x10_0000; /* MAP_FIXED which doesn't unmap underlying mapping */

        const MAP_UNINITIALIZED     = 0x400_0000; /* For anonymous mmap, memory could be uninitialized */

    }
}
