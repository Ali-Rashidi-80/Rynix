/* x86_64 SysV fiber swap (Linux / ELF SysV ABI).
 *
 * void rynix_rt_fiber_swap(RynixCtx *save, RynixCtx *restore);
 * Layout of RynixCtx (8 qwords): rbx, rbp, r12, r13, r14, r15, rsp, rip
 *
 * Windows MinGW uses the Microsoft x64 ABI — do not link this on Win32.
 */

#if defined(__linux__) && defined(__x86_64__) && !defined(_WIN32)

    .text
    .globl rynix_rt_fiber_swap
    .type rynix_rt_fiber_swap, @function
rynix_rt_fiber_swap:
    /* rdi = save, rsi = restore */
    movq %rbx, 0(%rdi)
    movq %rbp, 8(%rdi)
    movq %r12, 16(%rdi)
    movq %r13, 24(%rdi)
    movq %r14, 32(%rdi)
    movq %r15, 40(%rdi)
    movq %rsp, 48(%rdi)
    movq (%rsp), %rax          /* return address */
    movq %rax, 56(%rdi)

    movq 0(%rsi), %rbx
    movq 8(%rsi), %rbp
    movq 16(%rsi), %r12
    movq 24(%rsi), %r13
    movq 32(%rsi), %r14
    movq 40(%rsi), %r15
    movq 48(%rsi), %rsp
    movq 56(%rsi), %rax
    movq %rax, (%rsp)
    ret
    .size rynix_rt_fiber_swap, .-rynix_rt_fiber_swap

#endif
