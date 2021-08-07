use bitflags::*;

bitflags! {
    /// cloning flags
    pub struct CloneFlags: u64 {
        const CSIGNAL               = 0x0000_00ff;  /* signal mask to be sent at exit */
        const CLONE_VM              = 0x0000_0100;  /* set if VM shared between processes */
        const CLONE_FS              = 0x0000_0200;  /* set if fs info shared between processes */
        const CLONE_FILES           = 0x0000_0400;  /* set if open files shared between processes */
        const CLONE_SIGHAND         = 0x0000_0800;  /* set if signal handlers and blocked signals shared */
        const CLONE_PIDFD           = 0x0000_1000;  /* set if a pidfd should be placed in parent */
        const CLONE_PTRACE          = 0x0000_2000;  /* set if we want to let tracing continue on the child too */
        const CLONE_VFORK           = 0x0000_4000;  /* set if the parent wants the child to wake it up on mm_release */
        const CLONE_PARENT          = 0x0000_8000;  /* set if we want to have the same parent as the cloner */
        const CLONE_THREAD          = 0x0001_0000;  /* Same thread group? */
        const CLONE_NEWNS           = 0x0002_0000;  /* New mount namespace group */
        const CLONE_SYSVSEM         = 0x0004_0000;  /* share system V SEM_UNDO semantics */
        const CLONE_SETTLS          = 0x0008_0000;  /* create a new TLS for the child */
        const CLONE_PARENT_SETTID   = 0x0010_0000;  /* set the TID in the parent */
        const CLONE_CHILD_CLEARTID  = 0x0020_0000;  /* clear the TID in the child */
        const CLONE_DETACHED        = 0x0040_0000;  /* Unused, ignored */
        const CLONE_UNTRACED        = 0x0080_0000;  /* set if the tracing process can't force CLONE_PTRACE on this clone */
        const CLONE_CHILD_SETTID    = 0x0100_0000;  /* set the TID in the child */
        const CLONE_NEWCGROUP       = 0x0200_0000;  /* New cgroup namespace */
        const CLONE_NEWUTS          = 0x0400_0000;  /* New utsname namespace */
        const CLONE_NEWIPC          = 0x0800_0000;  /* New ipc namespace */
        const CLONE_NEWUSER         = 0x1000_0000;  /* New user namespace */
        const CLONE_NEWPID          = 0x2000_0000;  /* New pid namespace */
        const CLONE_NEWNET          = 0x4000_0000;  /* New network namespace */
        const CLONE_IO              = 0x8000_0000;  /* Clone io context */

        /* Flags for the clone3() syscall. */
        const CLONE_CLEAR_SIGHAND = 0x1_0000_0000;  /* Clear any signal handler and reset to SIG_DFL. */
        const CLONE_INTO_CGROUP   = 0x2_0000_0000;  /* Clone into a specific cgroup given the right permissions. */

        /*
         * cloning flags intersect with CSIGNAL so can be used with unshare and clone3
         * syscalls only:
         */
        const CLONE_NEWTIME         = 0x0000_0080;  /* New time namespace */
    }
}
