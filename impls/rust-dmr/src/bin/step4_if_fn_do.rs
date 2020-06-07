use rust_dmr_mal::interpreter::{PRINT, READ};
use rust_dmr_mal::types::{MalObject, MalSymbol};
use rust_dmr_mal::{cmdline, environment, evaluator, interpreter, printer, special_forms};

fn rep(line: &str, env: &mut environment::EnvironmentStack) -> printer::Result {
    let mut ctx = evaluator::Context {
        env,
        evaluator: EVAL,
    };
    PRINT(&READ(line).and_then(|ast| EVAL(&ast, &mut ctx).map_err(interpreter::Error::Eval)))
}

#[allow(non_snake_case)]
fn EVAL(ast: &MalObject, ctx: &mut evaluator::Context) -> evaluator::Result {
    evaluator::eval_ast_or_apply(ast, ctx, apply)
}

fn apply(argv: &[MalObject], ctx: &mut evaluator::Context) -> evaluator::Result {
    use MalObject::{Primitive, Symbol};
    log::debug!("apply {:?}", argv);
    if let Symbol(MalSymbol { name }) = &argv[0] {
        match name.as_str() {
            "def!" => return special_forms::apply_def(&argv[1..], ctx),
            "let*" => return special_forms::apply_let(&argv[1..], ctx),
            "do" => return special_forms::apply_do(&argv[1..], ctx),
            "if" => return special_forms::apply_if(&argv[1..], ctx),
            _ => (),
        };
    };
    let evaluated = evaluator::evaluate_sequence_elementwise(argv, ctx)?;
    let (callable, args) = evaluated.split_first().unwrap();
    match callable {
        Primitive(f) => evaluator::call_primitive(f, args),
        _ => panic!("apply: bad MalObject {:?}", evaluated),
    }
}

fn main() -> std::io::Result<()> {
    cmdline::run(rep)
}
