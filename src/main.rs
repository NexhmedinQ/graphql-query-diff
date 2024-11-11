use graphql_parser::{
    parse_query,
    query::{Document, Text},
};
use serde::{Deserialize, Serialize};
use std::{env, fs::File, io::Write};
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

#[derive(Serialize, Deserialize)]
struct Query {
    query: String,
    variables: Variables,
}

#[derive(Serialize, Deserialize)]
struct Variables {
    id: String,
}

#[derive(Debug)]
enum Operation {
    Delete,
    Insert,
    Nothing,
}

impl Operation {
    fn symbol(&self) -> &'static str {
        match self {
            Operation::Delete => "-",
            Operation::Insert => "+",
            Operation::Nothing => " ",
        }
    }

    fn color(&self) -> Color {
        match self {
            Operation::Delete => Color::Red,
            Operation::Insert => Color::Green,
            Operation::Nothing => Color::White,
        }
    }
}

#[derive(PartialEq)]
enum GraphqlOpType {
    Query,
    Subscription,
    Mutation,
}

#[derive(Debug)]
struct Printer<'a> {
    operation: Operation,
    line: &'a str,
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        panic!("Usage: cargo run <expected-query> <actual-query>")
    }

    let expected_query: Query =
        serde_json::from_reader(File::open(&args[1]).map_err(|err| err.to_string())?)
            .map_err(|err| err.to_string())?;
    let actual_query: Query =
        serde_json::from_reader(File::open(&args[2]).map_err(|err| err.to_string())?)
            .map_err(|err| err.to_string())?;

    let expected_ast: Document<&str> =
        parse_query(&expected_query.query).map_err(|err| err.to_string())?;
    let actual_ast: Document<&str> =
        parse_query(&actual_query.query).map_err(|err| err.to_string())?;

    let mut skip_line = 0;

    let expected_op = get_operation_type(&expected_ast.definitions[0]);
    let actual_op = get_operation_type(&actual_ast.definitions[0]);
    if actual_op == expected_op {
        skip_line = 1;
    }

    let parsed_expected = format!("{}", expected_ast);
    let parsed_actual = format!("{}", actual_ast);
    let parsed_expected_query = parsed_expected.split('\n').collect::<Vec<_>>();
    let parsed_actual_query = parsed_actual.split('\n').collect::<Vec<_>>();

    let coordinates = get_diff(
        &parsed_expected_query[skip_line..],
        &parsed_actual_query[skip_line..],
    );
    let mut print_ops: Vec<Printer> = Vec::new();
    if skip_line == 1 {
        print_ops.push(Printer {
            operation: Operation::Nothing,
            line: parsed_expected_query[0],
        });
    }
    print_output(
        coordinates,
        print_ops,
        &parsed_expected_query[skip_line..],
        &parsed_actual_query[skip_line..],
    );
    Ok(())
}

fn get_operation_type<'a, T: Text<'a>>(
    definitions: &graphql_parser::query::Definition<'a, T>,
) -> GraphqlOpType {
    match &definitions {
        graphql_parser::query::Definition::Operation(operation_definition) => {
            match operation_definition {
                graphql_parser::query::OperationDefinition::Query(_) => GraphqlOpType::Query,
                graphql_parser::query::OperationDefinition::Mutation(_) => GraphqlOpType::Mutation,
                graphql_parser::query::OperationDefinition::Subscription(_) => {
                    GraphqlOpType::Subscription
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
}

fn get_diff(expected: &[&str], actual: &[&str]) -> Vec<(usize, usize)> {
    let max_diff = expected.len() + actual.len();
    // 2D array
    let mut path = vec![vec![0; 2 * max_diff + 1]; max_diff];
    path[0][max_diff] = 0;

    for depth in 0..max_diff as i32 {
        for k in (-depth..=depth).step_by(2) {
            let index: usize = (max_diff as i32 + k).try_into().unwrap();
            if depth > 0 {
                path[depth as usize][index] = if k == -depth
                    || (k != depth
                        && path[(depth - 1) as usize][index - 1]
                            < path[(depth - 1) as usize][index + 1])
                {
                    path[(depth - 1) as usize][index + 1]
                } else {
                    path[(depth - 1) as usize][index - 1] + 1
                };
            }
            while path[depth as usize][index] < actual.len()
                && path[depth as usize][index] as i32 - k < expected.len() as i32
                && actual[path[usize::try_from(depth).unwrap()][index]]
                    == expected[usize::try_from(path[depth as usize][index] as i32 - k).unwrap()]
            {
                path[depth as usize][index] += 1;
            }
            if path[depth as usize][index] >= actual.len()
                && path[depth as usize][index] as i32 - k >= expected.len() as i32
            {
                return get_path(path, depth as usize, k, max_diff);
            }
        }
    }
    Vec::new()
}

fn get_path(
    traversal: Vec<Vec<usize>>,
    mut depth: usize,
    mut k: i32,
    max_diff: usize,
) -> Vec<(usize, usize)> {
    let mut path = Vec::new();
    let mut padded_k: usize = 0;
    while depth > 0 {
        padded_k = (max_diff as i32 + k).try_into().unwrap();
        let x = traversal[depth as usize][padded_k];
        let y: usize = (x as i32 - k).try_into().unwrap();
        path.push((x, y));
        if (k != depth as i32
            && traversal[depth - 1][padded_k + 1] >= traversal[depth - 1][padded_k - 1])
            || k == -i32::try_from(depth).unwrap()
        {
            k += 1;
        } else {
            k -= 1
        }
        depth -= 1;
    }
    path.push((
        traversal[0][padded_k + 1],
        traversal[0][padded_k + 1] - k as usize,
    ));
    path.reverse();
    path
}

fn print_output<'a>(
    coordinates: Vec<(usize, usize)>,
    mut print_ops: Vec<Printer<'a>>,
    expected: &[&'a str],
    actual: &[&'a str],
) {
    let mut prev_x: i32 = 0;
    let mut prev_y: i32 = 0;

    for (x, y) in coordinates {
        let (x, y) = (x as i32, y as i32);
        if x - y > prev_x - prev_y {
            print_ops.push(Printer {
                operation: Operation::Delete,
                line: actual[usize::try_from(prev_x).unwrap()],
            });
            prev_x += 1;
        } else if x - y < prev_x - prev_y {
            print_ops.push(Printer {
                operation: Operation::Insert,
                line: expected[usize::try_from(prev_y).unwrap()],
            });
            prev_y += 1;
        }
        while prev_x < x {
            print_ops.push(Printer {
                operation: Operation::Nothing,
                line: actual[usize::try_from(prev_x).unwrap()],
            });
            prev_x += 1;
            prev_y += 1;
        }
    }
    let mut colour_spec = ColorSpec::new();
    let bufwtr = BufferWriter::stdout(ColorChoice::Always);

    for op in print_ops {
        colour_spec.set_fg(Some(op.operation.color()));
        let mut buffer = bufwtr.buffer();
        buffer.set_color(&colour_spec).unwrap();
        writeln!(&mut buffer, "{} {}", op.operation.symbol(), op.line).unwrap();
        bufwtr.print(&buffer).unwrap();
    }
}
