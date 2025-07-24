return value allocated by caller and passed in rax (unless <= 8 bytes then passed in rax)
every register (except rax when sizeof(retval) <= 8 bytes) saved by callee
arguments put on the stack by caller and pointer passed in rdi (optimization for owned single parameters or smth)
