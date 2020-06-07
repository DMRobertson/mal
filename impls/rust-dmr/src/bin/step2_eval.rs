use rust_dmr_mal::interpreter::{PRINT, READ};
use rust_dmr_mal::types::{MalList, MalObject};
use rust_dmr_mal::{cmdline, environment, evaluator, interpreter, printer};

fn rep(line: &str, env: &mut environment::EnvironmentStack) -> printer::Result {
    PRINT(&READ(line).and_then(|ast| EVAL(&ast, env).map_err(interpreter::Error::Eval)))
}

#[allow(non_snake_case)]
fn EVAL(ast: &MalObject, env: &mut environment::EnvironmentStack) -> evaluator::Result {
    use MalObject::List;
    log::debug!("eval {:?}", ast);

    // Not the biggest fan of this. Wanted a way to call back to EVAL while still keeping the boring bits that don't change between steps in the library.
    let mut ctx = evaluator::Context {
        env,
        evaluator: EVAL,
    };

    let result = match ast {
        List(list) => match list.len() {
            0 => Ok(List(MalList::new())),
            _ => apply(list, &mut ctx),
        },
        _ => evaluator::evaluate_ast(ast, &mut ctx),
    };
    log::debug!("eval produced {:?}", result);
    result
}

fn apply(argv: &[MalObject], ctx: &mut evaluator::Context) -> evaluator::Result {
    use MalObject::{Integer, PrimitiveBinaryOp};
    log::debug!("apply {:?}", argv);
    let evaluated = evaluator::evaluate_sequence_elementwise(argv, ctx)?;
    match &evaluated[0] {
        PrimitiveBinaryOp(op) => match evaluated[1..] {
            [Integer(x), Integer(y)] => Ok(Integer(op(x, y))),
            _ => panic!("apply: bad PrimitiveBinaryOp"),
        },
        _ => panic!("apply: bad MalObject {:?}", evaluated),
    }
}

fn main() -> std::io::Result<()> {
    cmdline::run(rep)
}
