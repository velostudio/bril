mod rt;
pub mod translator;

use crate::translator::{find_func, Translator};
use bril_rs as bril;
use bril_rs::Program;
use cranelift_jit::JITModule;
use cranelift_object::ObjectModule;

/// Compile a program ahead of time to an object file.
///
/// * `program` - the Bril program to compile
/// * `target` - the target triple, or None to target the host
/// * `output` - the filename where we should write the object file
/// * `opt_level` - a Cranelift optimization level
/// * `dump_ir` - optionally emit the Cranelift IR to stdout
pub fn compile(
    program: &Program,
    target: Option<String>,
    output: &str,
    opt_level: &str,
    dump_ir: bool,
) {
    // Compile.
    let mut trans = Translator::<ObjectModule>::new(target, opt_level);
    trans.compile_prog(program, dump_ir);

    // Add a C-style `main` wrapper.
    let Some(main) = find_func(&program.functions, "main") else {
        return;
    };

    trans.add_c_main(&main.args, dump_ir);

    // Write object file.
    trans.emit(output);
}

/// Just-in-time compile and execute a Bril program.
///
/// * `program` - the Bril program to compile
/// * `args` - the arguments to pass to the `@main` function
/// * `dump_ir` - optionally emit the Cranelift IR to stdout
pub fn jit_run(trans: &mut Translator<JITModule>, program: &Program, args: Vec<String>, dump_ir: bool, swap: bool) {
    // Compile.

    if swap {
        trans.prepare_for_function_redefine(program);
        trans.compile_prog(program, dump_ir);
        trans.finalize_definitions();

        // FIXME: Ideally, to test that jit works as expected would be cool to do hot swapping in dedicated
        // thread and have endlessly running main function on main thread that uses hotswapped function
        //
        // Invoke the main function.
        let Some(_main) = find_func(&program.functions, "main") else {
            return;
        };
        unsafe { trans.run_func_by_name("main".to_string()) };
    } else {
        trans.compile_prog(program, dump_ir);
        // Add a JIT wrapper for `main`.
        let Some(main) = find_func(&program.functions, "main") else {
            return;
        };
        let entry_id = trans.add_mem_wrapper("main", &main.args, dump_ir);

        // Parse CLI arguments.
        if main.args.len() != args.len() {
            panic!(
                "@main expects {} arguments; got {}",
                main.args.len(),
                args.len()
            );
        }
        let main_args: Vec<bril::Literal> = main
            .args
            .iter()
            .zip(args)
            .map(|(arg, val_str)| match arg.arg_type {
                bril::Type::Int => bril::Literal::Int(val_str.parse().unwrap()),
                bril::Type::Bool => bril::Literal::Bool(val_str == "true"),
                bril::Type::Float => bril::Literal::Float(val_str.parse().unwrap()),
                bril::Type::Char => bril::Literal::Char(val_str.parse().unwrap()),
                bril::Type::Pointer(_) => unimplemented!("pointers not supported as main args"),
            })
            .collect();

        // Invoke the main function.
        unsafe { trans.run(entry_id, &main_args) };
    }
}

/// The C runtime library for Rust library users.
pub fn c_runtime() -> &'static str {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/rt.c"))
}
