#include <stdlib.h>
#include <sys/ptrace.h>
#include <sys/types.h>

long c_ptrace_peekdata(pid_t child, long address) {
  return ptrace(PTRACE_PEEKDATA, child, address, NULL);
}
