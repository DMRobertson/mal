use rust_dmr_mal::interpreter::{PRINT, READ};
use rust_dmr_mal::types::MalObject;
use rust_dmr_mal::{cmdline, environment, evaluator, interpreter, printer};

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
    use MalObject::Primitive;
    log::debug!("apply {:?}", argv);
    let evaluated = evaluator::evaluate_sequence_elementwise(argv, ctx)?;
    match &evaluated[0] {
        Primitive(f) => evaluator::call_primitive(f, &evaluated[1..]),
        _ => panic!("apply: bad MalObject {:?}", evaluated),
    }
}

fn main() -> std::io::Result<()> {
    cmdline::run(rep)
}
