extern "C" {
    pub fn stext();
    pub fn strampoline();
    pub fn etext();
    pub fn srodata();
    pub fn erodata();
    pub fn sdata();
    pub fn edata();
    pub fn boot_stack_lower_bound(); // stack lower bound
    pub fn boot_stack_upper_bound(); // stack top
    pub fn sbss_with_stack();
    pub fn sbss();
    pub fn ebss();
    pub fn ekernel();

    pub fn _num_app();
}
