// AIOS Interrupt Descriptor Table (IDT)
//
// Model: claude-sonnet-4-6
// Tool: claude-code
// Prompt: Fix IDT init so timer handler is loaded into the live IDT; fix PIC initialization;
//         add int 0x80 syscall handler that dispatches to handle_syscall via GPR save/restore.

use core::sync::atomic::{AtomicBool, Ordering};
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::instructions::port::PortReadOnly;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

static PICS: Mutex<Option<ChainedPics>> = Mutex::new(None);

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub static TIMER_TICK: AtomicBool = AtomicBool::new(false);

/// Set by sys_exit; syscall_dispatch longjmps back to the shell after handle_syscall returns.
pub static PROCESS_EXITED: AtomicBool = AtomicBool::new(false);

extern "C" {
    // Top of the 64 KiB boot stack exported from boot.S (same address used by TSS).
    static boot_stack_top: u8;
}

// Raw syscall trampoline: saves caller-saved GPRs, calls syscall_dispatch,
// restores GPRs, and returns via iretq. Registered via set_handler_addr so
// the IDT does not impose the x86-interrupt ABI on it.
//
// Linux x86_64 syscall convention via int 0x80 (32-bit compat) reuses:
//   rax = syscall number, rdi = arg1, rsi = arg2, rdx = arg3
// We honour the same layout so a static ELF built for this kernel works.
core::arch::global_asm!(
    ".global syscall_int80_trampoline",
    "syscall_int80_trampoline:",
    // DEBUG: Write '*' to COM1 (0x3F8) to signal int 0x80 entry
    "push rax",
    "push rdx",
    "mov al, 0x2A",  // '*' character
    "mov dx, 0x3F8", // COM1 port
    "out dx, al",
    "pop rdx",
    "pop rax",
    "push rbp",
    "push rbx",
    "push r10",
    "push r11",
    "push r12",
    "push r13",
    "push r14",
    "push r15",
    // rax=syscall_num, rdi=arg1, rsi=arg2, rdx=arg3 already in place per ABI
    "call syscall_dispatch",
    // result returned in rax by syscall_dispatch
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop r11",
    "pop r10",
    "pop rbx",
    "pop rbp",
    "iretq",
);

// Native syscall trampoline: handles the `syscall` instruction from ring 3.
// On entry: rcx=user RIP (saved by CPU), r11=user RFLAGS (saved by CPU),
//           rax=syscall number, rdi=arg1, rsi=arg2, rdx=arg3, r10=arg4.
// We preserve rcx/r11 across the dispatch call since sysretq restores
// RIP from rcx and RFLAGS from r11.
core::arch::global_asm!(
    ".global syscall_native_trampoline",
    "syscall_native_trampoline:",
    // rcx = user RIP (must survive until sysretq)
    // r11 = user RFLAGS (must survive until sysretq)
    // Save all registers that syscall_dispatch (extern "C") may clobber,
    // except rax which carries the return value.
    "push rcx", // user RIP — restored by sysretq
    "push r11", // user RFLAGS — restored by sysretq
    "push rbp",
    "push rbx",
    "push r12",
    "push r13",
    "push r14",
    "push r15",
    // rax=num, rdi=arg1, rsi=arg2, rdx=arg3 already match syscall_dispatch ABI
    "call syscall_dispatch",
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop rbx",
    "pop rbp",
    "pop r11", // restore user RFLAGS for sysretq
    "pop rcx", // restore user RIP for sysretq
    "sysretq",
);

/// Called from the int 0x80 trampoline with syscall registers already in place.
/// Returns the syscall result in rax (via normal Rust return convention).
#[no_mangle]
pub extern "C" fn syscall_dispatch(
    _unused: u64, // placeholder — the real args arrive in rdi/rsi/rdx/rax
) -> i64 {
    // Read syscall arguments from registers via inline asm.
    let (num, arg1, arg2, arg3): (usize, usize, usize, usize);
    // # Safety
    // Reading rax/rdi/rsi/rdx here is safe: we are in the syscall trampoline
    // context where these registers contain the syscall number and arguments
    // placed there by the user-mode caller before executing `int 0x80`.
    // No memory is read or written; only register values are captured.
    unsafe {
        core::arch::asm!(
            "mov {num}, rax",
            "mov {a1}, rdi",
            "mov {a2}, rsi",
            "mov {a3}, rdx",
            num = out(reg) num,
            a1  = out(reg) arg1,
            a2  = out(reg) arg2,
            a3  = out(reg) arg3,
        );
    }
    let result = crate::syscalls::handle_syscall(num, arg1, arg2, arg3);

    // If the process called exit, re-enter the shell on a fresh kernel stack.
    // handle_syscall already released the SYSCALL_MANAGER mutex before returning.
    // We must NOT touch the current (trampoline) stack since it may be partially
    // overwritten; instead reset RSP to boot_stack_top and jmp to the shell loop.
    if PROCESS_EXITED.swap(false, Ordering::AcqRel) {
        // # Safety
        // Resetting RSP to boot_stack_top (top of the 64 KiB kernel stack) gives us
        // a clean stack. jmp (not call) to shell_prompt_loop so no return address is
        // pushed; shell_prompt_loop runs the command loop until the kernel stops.
        unsafe {
            let stack_top = &boot_stack_top as *const u8 as u64;
            core::arch::asm!(
                "mov rsp, {stack}",
                "jmp {resume}",
                stack = in(reg) stack_top,
                resume = sym crate::shell::shell_prompt_loop,
                options(noreturn)
            );
        }
    }

    result as i64
}

pub fn init() {
    // # Safety
    // `IDT` is a `static mut` which normally risks aliased mutable references.
    // This is safe here because:
    //   1. `init()` runs exactly once during single-threaded boot, before
    //      interrupts are enabled (no concurrent access is possible).
    //   2. After `IDT.load()` the IDT is only read by the CPU's interrupt
    //      dispatch mechanism, never written again — so no aliasing occurs
    //      at runtime.
    unsafe {
        IDT.breakpoint.set_handler_fn(breakpoint_handler);
        IDT.double_fault.set_handler_fn(double_fault_handler);
        IDT.page_fault.set_handler_fn(page_fault_handler);
        IDT.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        IDT.stack_segment_fault
            .set_handler_fn(stack_segment_fault_handler);
        // IRQ 0 (timer) → vector PIC_1_OFFSET (32), IRQ 1 (keyboard) → 33
        IDT[PIC_1_OFFSET].set_handler_fn(timer_interrupt_handler);
        IDT[PIC_1_OFFSET + 1].set_handler_fn(keyboard_interrupt_handler);
        // int 0x80 → syscall trampoline (vector 0x80 = 128)
        // set_handler_addr bypasses the x86-interrupt ABI so our trampoline
        // manages its own register save/restore and iretq.
        extern "C" {
            fn syscall_int80_trampoline();
        }
        IDT[0x80u8]
            .set_handler_addr(x86_64::VirtAddr::new(
                syscall_int80_trampoline as *const () as usize as u64,
            ))
            .set_privilege_level(x86_64::PrivilegeLevel::Ring3);
        IDT.load();
    }

    init_pic();
    configure_pit_timer();
}

// Kept for compatibility with kernel_entry call sequence; IDT is fully set up in init().
pub fn init_idt() {}

/// Configure SYSCALL/SYSRET MSRs so the `syscall` instruction from ring 3 dispatches here.
///
/// STAR layout:
///   [63:48] = 0x10 (kernel_data) → sysretq loads SS=0x18|3 (user_data), CS=0x20|3 (user_code)
///   [47:32] = 0x08 (kernel_code) → syscall loads CS=0x08, SS=0x10
/// Requires GDT order: null, kernel_code(0x08), kernel_data(0x10), user_data(0x18), user_code(0x20)
pub fn init_syscall() {
    extern "C" {
        fn syscall_native_trampoline();
    }

    // # Safety
    // Writing standard x86_64 syscall MSRs during single-threaded boot init.
    // All MSR addresses are documented in the Intel/AMD SDMs.
    unsafe {
        // Enable SCE bit in IA32_EFER (0xC000_0080)
        let efer_lo: u32;
        let efer_hi: u32;
        core::arch::asm!("rdmsr", in("ecx") 0xC000_0080u32, out("eax") efer_lo, out("edx") efer_hi);
        let efer: u64 = ((efer_hi as u64) << 32) | (efer_lo as u64) | 1; // set SCE
        core::arch::asm!("wrmsr", in("ecx") 0xC000_0080u32, in("eax") efer as u32, in("edx") (efer >> 32) as u32);

        // STAR: [63:48]=0x0010 (for sysretq), [47:32]=0x0008 (for syscall entry)
        let star: u64 = (0x0010u64 << 48) | (0x0008u64 << 32);
        core::arch::asm!("wrmsr", in("ecx") 0xC000_0081u32, in("eax") star as u32, in("edx") (star >> 32) as u32);

        // LSTAR: 64-bit syscall handler address
        let lstar = syscall_native_trampoline as usize as u64;
        core::arch::asm!("wrmsr", in("ecx") 0xC000_0082u32, in("eax") lstar as u32, in("edx") (lstar >> 32) as u32);

        // SFMASK: clear IF (bit 9) on syscall entry
        let sfmask: u64 = 0x200;
        core::arch::asm!("wrmsr", in("ecx") 0xC000_0084u32, in("eax") sfmask as u32, in("edx") (sfmask >> 32) as u32);
    }
}

pub fn end_of_interrupt(irq: u8) {
    let mut pics_guard = PICS.lock();
    if let Some(ref mut pics) = *pics_guard {
        // # Safety
        // notify_end_of_interrupt is safe when called with a valid IRQ vector that
        // was delivered by the PIC hardware.
        unsafe {
            pics.notify_end_of_interrupt(irq);
        }
    }
}

fn init_pic() {
    let mut pics_guard = PICS.lock();
    if pics_guard.is_none() {
        let mut pics = unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) };
        // # Safety
        // initialize() programs the 8259 PIC hardware. Called once at boot before
        // interrupts are enabled. No other code touches the PIC ports at this point.
        unsafe { pics.initialize() };
        *pics_guard = Some(pics);
    }
}

fn configure_pit_timer() {
    // PIT runs at ~1193182 Hz; divisor 11931 → ~100 Hz (10 ms ticks).
    let divisor: u16 = 11931;
    // # Safety
    // Writing to PIT ports 0x43/0x40 is standard x86 practice during boot.
    unsafe {
        use x86_64::instructions::port::Port;
        let mut cmd: Port<u8> = Port::new(0x43);
        let mut data: Port<u8> = Port::new(0x40);
        cmd.write(0x36); // channel 0, lobyte/hibyte, mode 3
        data.write((divisor & 0xFF) as u8);
        data.write(((divisor >> 8) & 0xFF) as u8);
    }
}

extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    loop {
        // # Safety
        // HLT in a double-fault handler is safe; we cannot recover so we halt.
        unsafe { core::arch::asm!("hlt") }
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    TIMER_TICK.store(true, Ordering::SeqCst);
    end_of_interrupt(PIC_1_OFFSET);
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut data = PortReadOnly::<u8>::new(0x60);
    let _scan_code: u8 = unsafe { data.read() };
    end_of_interrupt(PIC_1_OFFSET + 1);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: x86_64::structures::idt::PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    crate::serial::write_str("\r\n[FAULT] Page Fault!\r\n");
    crate::serial::write_str("  Faulting address (CR2): 0x");
    // # Safety
    // Reading CR2 after a page fault is safe; it contains the faulting address.
    let cr2 = Cr2::read_raw();
    print_hex(cr2);
    crate::serial::write_str("\r\n  Error code: ");
    print_hex(error_code.bits());
    crate::serial::write_str("\r\n    ");
    if error_code.contains(x86_64::structures::idt::PageFaultErrorCode::PROTECTION_VIOLATION) {
        crate::serial::write_str("PROTECTION_VIOLATION ");
    } else {
        crate::serial::write_str("NOT_PRESENT ");
    }
    if error_code.contains(x86_64::structures::idt::PageFaultErrorCode::CAUSED_BY_WRITE) {
        crate::serial::write_str("WRITE ");
    } else {
        crate::serial::write_str("READ ");
    }
    if error_code.contains(x86_64::structures::idt::PageFaultErrorCode::USER_MODE) {
        crate::serial::write_str("USER_MODE ");
    } else {
        crate::serial::write_str("KERNEL_MODE ");
    }
    if error_code.contains(x86_64::structures::idt::PageFaultErrorCode::INSTRUCTION_FETCH) {
        crate::serial::write_str("INSTRUCTION_FETCH");
    }
    crate::serial::write_str("\r\n  RIP: 0x");
    print_hex(stack_frame.instruction_pointer.as_u64());
    crate::serial::write_str("\r\n  RSP: 0x");
    print_hex(stack_frame.stack_pointer.as_u64());
    crate::serial::write_str("\r\n  CS: 0x");
    print_hex(stack_frame.code_segment.0 as u64);
    crate::serial::write_str("\r\n");

    loop {
        // # Safety
        // HLT in a fault handler is safe; we halt to prevent further damage.
        unsafe { core::arch::asm!("hlt") }
    }
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    crate::serial::write_str("\r\n[FAULT] General Protection Fault!\r\n");
    crate::serial::write_str("  Error code: 0x");
    print_hex(error_code);
    crate::serial::write_str("\r\n  RIP: 0x");
    print_hex(stack_frame.instruction_pointer.as_u64());
    crate::serial::write_str("\r\n  RSP: 0x");
    print_hex(stack_frame.stack_pointer.as_u64());
    crate::serial::write_str("\r\n  CS: 0x");
    print_hex(stack_frame.code_segment.0 as u64);
    crate::serial::write_str("\r\n  RFLAGS: 0x");
    print_hex(stack_frame.cpu_flags.bits());
    crate::serial::write_str("\r\n");

    if error_code != 0 {
        let external = (error_code & 1) != 0;
        let table = (error_code >> 1) & 0x3;
        let index = (error_code >> 3) & 0x1FFF;
        crate::serial::write_str("  Selector: ");
        if external {
            crate::serial::write_str("external ");
        }
        crate::serial::write_str("table=");
        print_hex(table);
        crate::serial::write_str(" index=");
        print_hex(index);
        crate::serial::write_str("\r\n");
    }

    loop {
        // # Safety
        // HLT in a fault handler is safe; we halt to prevent further damage.
        unsafe { core::arch::asm!("hlt") }
    }
}

extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    crate::serial::write_str("\r\n[FAULT] Stack Segment Fault!\r\n");
    crate::serial::write_str("  Error code: 0x");
    print_hex(error_code);
    crate::serial::write_str("\r\n  RIP: 0x");
    print_hex(stack_frame.instruction_pointer.as_u64());
    crate::serial::write_str("\r\n  RSP: 0x");
    print_hex(stack_frame.stack_pointer.as_u64());
    crate::serial::write_str("\r\n");

    loop {
        // # Safety
        // HLT in a fault handler is safe; we halt to prevent further damage.
        unsafe { core::arch::asm!("hlt") }
    }
}

fn print_hex(val: u64) {
    let hex_chars = b"0123456789abcdef";
    let mut buf = [0u8; 16];
    for (i, item) in buf.iter_mut().enumerate() {
        let nibble = ((val >> (60 - i * 4)) & 0xF) as usize;
        *item = hex_chars[nibble];
    }
    for &byte in &buf {
        crate::serial::write_byte(byte);
    }
}

pub fn enable_interrupts() {
    // # Safety
    // Enabling interrupts is safe after all handlers are registered via init().
    unsafe { core::arch::asm!("sti") }
}

pub fn disable_interrupts() {
    // # Safety
    // Disabling interrupts is safe; used during critical sections.
    unsafe { core::arch::asm!("cli") }
}

pub fn is_timer_tick() -> bool {
    TIMER_TICK.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_vector_offsets() {
        assert_eq!(PIC_1_OFFSET, 32);
        assert_eq!(PIC_1_OFFSET + 1, 33);
        assert_eq!(PIC_2_OFFSET, 40);
    }

    #[test]
    fn test_exception_handler_signature() {
        extern "x86-interrupt" fn test_handler(_: InterruptStackFrame) {}
        let _: extern "x86-interrupt" fn(InterruptStackFrame) = test_handler;
    }

    #[test]
    fn test_pit_divisor_calculation() {
        let divisor: u16 = 11931;
        assert!(divisor > 0);
        let frequency = 1193182u32 / u32::from(divisor);
        assert_eq!(frequency, 100);
    }

    #[test]
    fn test_timer_tick_atomic_flag() {
        TIMER_TICK.store(false, Ordering::SeqCst);
        assert!(!TIMER_TICK.load(Ordering::SeqCst));
        TIMER_TICK.store(true, Ordering::SeqCst);
        assert!(TIMER_TICK.load(Ordering::SeqCst));
        let tick = TIMER_TICK.swap(false, Ordering::SeqCst);
        assert!(tick);
        assert!(!TIMER_TICK.load(Ordering::SeqCst));
    }

    #[test]
    fn test_pic_offsets_sequential() {
        assert_eq!(PIC_2_OFFSET, PIC_1_OFFSET + 8);
    }

    #[test]
    fn test_timer_interrupt_vector() {
        assert_eq!(PIC_1_OFFSET as usize, 32);
    }

    #[test]
    fn test_keyboard_interrupt_vector() {
        assert_eq!(PIC_1_OFFSET as usize + 1, 33);
    }

    #[test]
    fn test_syscall_vector() {
        assert_eq!(0x80u8 as usize, 128);
    }

    #[test]
    fn test_syscall_dispatch_exists() {
        // Verify the dispatch function is callable (not dead-stripped).
        let _ = syscall_dispatch as unsafe extern "C" fn(u64) -> i64;
    }
}
