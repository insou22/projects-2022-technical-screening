use std::ops::Deref;

use neon::prelude::*;

#[derive(PartialEq)]
enum Sign {
    PositiveIsh,
    NegativeIsh,
}

impl Sign {
    fn flip(&self) -> Self {
        match self {
            Self::PositiveIsh => Self::NegativeIsh,
            Self::NegativeIsh => Self::PositiveIsh,
        }
    }
}

fn number_sign<'a>(cx: &mut impl Context<'a>, number: &Handle<JsNumber>) -> Sign {
    let value = number.value(cx);

    if value.is_sign_positive() || value == 0.0 {
        Sign::PositiveIsh
    } else {
        Sign::NegativeIsh
    }
}

fn alt_numbers(mut cx: FunctionContext) -> JsResult<JsArray> {
    let mut input = cx.argument::<JsArray>(0)?.deref().to_vec(&mut cx)?
        .into_iter()
        .map(|value| value.downcast::<JsNumber, _>(&mut cx).unwrap())
        .collect::<Vec<_>>();

    let n_positive_ish = input.iter()
        .filter(|number| number_sign(&mut cx, number) == Sign::PositiveIsh)
        .count();
    let n_negative_ish = input.len() - n_positive_ish;

    let mut current_sign = if n_positive_ish >= n_negative_ish {
        Sign::PositiveIsh
    } else {
        Sign::NegativeIsh
    };

    let output = cx.empty_array();

    while !input.is_empty() {
        let (idx, _) = input.iter()
            .enumerate()
            .find(|(_, number)| number_sign(&mut cx, number) == current_sign)
            .expect("constraint on the exercise");

        let value = input.remove(idx);

        let output_len = output.len(&mut cx);
        output.set(&mut cx, output_len, value)?;

        current_sign = current_sign.flip();
    }

    Ok(output)
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("altNumbers", alt_numbers)?;
    Ok(())
}
