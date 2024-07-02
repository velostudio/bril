use argh::FromArgs;
use bril::{output_abstract_program_to_buffer, Program};
use bril2json::parse_abstract_program;
use bril_rs as bril;
use brilift::{compile, jit_run};
use std::{str::FromStr, thread, time::Duration};

#[derive(FromArgs)]
#[argh(description = "Bril compiler")]
struct BriliftArgs {
    #[argh(switch, short = 'j', description = "JIT and run")]
    jit: bool,

    #[argh(option, short = 'f', description = "filepath to bril program")]
    filepath: Option<String>,

    #[argh(option, short = 't', description = "target triple")]
    target: Option<String>,

    #[argh(
        option,
        short = 'o',
        description = "output object file",
        default = "String::from(\"bril.o\")"
    )]
    output: String,

    #[argh(switch, short = 'd', description = "dump CLIF IR")]
    dump_ir: bool,

    #[argh(switch, short = 'v', description = "verbose logging")]
    verbose: bool,

    #[argh(
        option,
        short = 'O',
        description = "optimization level (none, speed, or speed_and_size)",
        default = "OptLevel::None"
    )]
    opt_level: OptLevel,

    #[argh(
        positional,
        description = "arguments for @main function (JIT mode only)"
    )]
    args: Vec<String>,
}

pub enum OptLevel {
    None,
    Speed,
    SpeedAndSize,
}

impl OptLevel {
    pub fn to_str(self) -> &'static str {
        match self {
            OptLevel::None => "none",
            OptLevel::Speed => "speed",
            OptLevel::SpeedAndSize => "speed_and_size",
        }
    }
}

impl FromStr for OptLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<OptLevel, String> {
        match s {
            "none" => Ok(OptLevel::None),
            "speed" => Ok(OptLevel::Speed),
            "speed_and_size" => Ok(OptLevel::SpeedAndSize),
            _ => Err(format!("unknown optimization level {s}")),
        }
    }
}

fn main() {
    let args: BriliftArgs = argh::from_env();

    // Set up logging.
    simplelog::TermLogger::init(
        if args.verbose {
            simplelog::LevelFilter::Debug
        } else {
            simplelog::LevelFilter::Warn
        },
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    // Load the Bril program from file.
    let filepath = args.filepath.unwrap();
    let prog = get_prog(filepath.clone());

    if args.jit {
        jit_run(&prog, args.args.clone(), args.dump_ir, false);
        for i in 1..1000 {
            println!("hotswap in 1 seconds, iteration {}", i);
            thread::sleep(Duration::from_millis(1000));
            let prog = get_prog(filepath.clone());
            jit_run(&prog, args.args.clone(), args.dump_ir, true);
        }
    } else {
        compile(
            &prog,
            args.target.clone(),
            &args.output,
            args.opt_level.to_str(),
            args.dump_ir,
        );
    }
}

fn get_prog(filepath: String) -> Program {
    let program = parse_abstract_program(false, false, Some(filepath));
    let buffer = output_abstract_program_to_buffer(&program);
    bril::load_program_from_buffer(&buffer)
}
