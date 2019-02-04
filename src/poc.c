#include <stdio.h>
#include <sys/ptrace.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#include <sys/reg.h>   /* For constants ORIG_EAX etc */

extern void peekuser(int child) {
  long orig_rax;
  printf("C: offset: %i\n", 8 * ORIG_RAX);
  orig_rax = ptrace(PTRACE_PEEKUSER, child, 8 * ORIG_RAX, NULL);
  printf("C: The child made a system call %ld\n", orig_rax);
}

int main_() {
  pid_t child;
  child = fork();
  if(child == 0) {
    ptrace(PTRACE_TRACEME, 0, NULL, NULL);
    execl("/bin/ls", "ls", NULL);
  }
  else {
    wait(NULL);
    peekuser(child);
    ptrace(PTRACE_CONT, child, NULL, NULL);
  }
  return 0;
}
