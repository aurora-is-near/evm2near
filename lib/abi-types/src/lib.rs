// This is free and unencumbered software released into the public domain.

use ethabi::ParamType;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit0, multispace0},
    combinator::all_consuming,
    multi::{fold_many0, separated_list1},
    sequence::{delimited, tuple},
};
use std::borrow::Cow;

pub use ethabi;

pub fn parse_param_type(input: &str) -> Result<ParamType, ParseError> {
    let (_, typ) = all_consuming(inner_parse_param_type)(input).map_err(|e| match e {
        nom::Err::Error(e) => e,
        nom::Err::Failure(e) => e,
        nom::Err::Incomplete(_) => unreachable!(),
    })?;
    Ok(typ)
}

#[derive(Debug, PartialEq)]
pub enum ParseError<'a> {
    Syntax(nom::error::Error<&'a str>),
    Logic {
        input: &'a str,
        message: Cow<'static, str>,
    },
}

fn inner_parse_param_type(input: &str) -> IResult<ParamType> {
    // skip leading whitespace
    let (remainder, _) = multispace0(input)?;
    let (remainder, typ) = alt((
        parse_tuple,
        number_type_parser("int", ParamType::Int),
        number_type_parser("uint", ParamType::Uint),
        parse_basic_type,
    ))(remainder)?;
    // recursively see if there are any array indicators after the type
    let (remainder, final_type) = fold_many0(
        parse_array_suffix,
        || typ.clone(),
        |acc_type, maybe_size| match maybe_size {
            None => ParamType::Array(Box::new(acc_type)),
            Some(size) => ParamType::FixedArray(Box::new(acc_type), size),
        },
    )(remainder)?;
    // skip trailing whitespace
    let (remainder, _) = multispace0(remainder)?;
    Ok((remainder, final_type))
}

fn parse_array_suffix(input: &str) -> IResult<Option<usize>> {
    let (remainder, maybe_size) = delimited(tag("["), digit0, tag("]"))(input)?;
    if maybe_size.is_empty() {
        Ok((remainder, None))
    } else {
        let (_, fixed_size) = nom::character::complete::u32(maybe_size)?;
        Ok((remainder, Some(fixed_size as usize)))
    }
}

fn parse_tuple(input: &str) -> IResult<ParamType> {
    let (remainder, inner) = delimited(
        tag("("),
        separated_list1(tag(","), inner_parse_param_type),
        tag(")"),
    )(input)?;
    Ok((remainder, ParamType::Tuple(inner)))
}

fn number_type_parser<'a, F>(
    name: &'static str,
    as_type: F,
) -> impl FnMut(&'a str) -> IResult<ParamType>
where
    F: Fn(usize) -> ParamType,
{
    move |input: &'a str| -> IResult<ParamType> {
        let (remainder, (_, size)) = tuple((tag(name), nom::character::complete::u16))(input)?;
        if size == 0 {
            return Err(nom_error_with_message(
                input,
                format!("{}0 is not a type", name),
            ));
        } else if size % 8 != 0 {
            return Err(nom_error_with_message(
                input,
                format!("{} sizes are in steps of 8", name),
            ));
        } else if size > 256 {
            return Err(nom_error_with_message(
                input,
                format!("{}256 is the largest number type", name),
            ));
        }
        let typ = as_type(size as usize);
        Ok((remainder, typ))
    }
}

fn parse_basic_type(input: &str) -> IResult<ParamType> {
    let (remainder, name) = alt((
        tag("address"),
        tag("bytes"),
        tag("bool"),
        tag("string"),
        tag("int"),
        tag("uint"),
    ))(input)?;

    let typ = match name {
        "address" => ethabi::ParamType::Address,
        "bytes" => {
            let (remainder, maybe_size) = digit0(remainder)?;
            if !maybe_size.is_empty() {
                let (_, fixed_size) = nom::character::complete::u8(maybe_size)?;
                if fixed_size > 32 {
                    return Err(nom_error_with_static_message(
                        input,
                        "Fixed-sized byte arrays cannot exceed 32 bytes",
                    ));
                } else {
                    return Ok((remainder, ParamType::FixedBytes(fixed_size as usize)));
                }
            }
            ParamType::Bytes
        }
        "bool" => ethabi::ParamType::Bool,
        "string" => ethabi::ParamType::String,
        "int" => ethabi::ParamType::Int(256),
        "uint" => ethabi::ParamType::Uint(256),
        _ => unreachable!(),
    };

    Ok((remainder, typ))
}

impl<'a> nom::error::ParseError<&'a str> for ParseError<'a> {
    fn from_error_kind(input: &'a str, kind: nom::error::ErrorKind) -> Self {
        Self::Syntax(nom::error::Error { input, code: kind })
    }

    fn append(input: &'a str, kind: nom::error::ErrorKind, other: Self) -> Self {
        match other {
            Self::Syntax(e) => Self::Syntax(nom::error::ParseError::append(input, kind, e)),
            other => other,
        }
    }
}

fn nom_error_with_static_message<'a>(
    input: &'a str,
    message: &'static str,
) -> nom::Err<ParseError<'a>> {
    nom::Err::Failure(ParseError::Logic {
        input,
        message: Cow::Borrowed(message),
    })
}

fn nom_error_with_message(input: &str, message: String) -> nom::Err<ParseError> {
    nom::Err::Failure(ParseError::Logic {
        input,
        message: Cow::Owned(message),
    })
}

type IResult<'a, T> = Result<(&'a str, T), nom::Err<ParseError<'a>>>;

#[cfg(test)]
mod tests {
    use super::{parse_param_type, ParseError};
    use ethabi::ParamType;

    #[test]
    fn test_parse_basic_types() {
        assert_eq!(parse_param_type("address").unwrap(), ParamType::Address);
        assert_eq!(parse_param_type("bytes").unwrap(), ParamType::Bytes);
        assert_eq!(
            parse_param_type("bytes32").unwrap(),
            ParamType::FixedBytes(32)
        );
        assert_eq!(parse_param_type("bool").unwrap(), ParamType::Bool);
        assert_eq!(parse_param_type("string").unwrap(), ParamType::String);
        assert_eq!(parse_param_type("int").unwrap(), ParamType::Int(256));
        assert_eq!(parse_param_type("uint").unwrap(), ParamType::Uint(256));
        assert_eq!(parse_param_type("int32").unwrap(), ParamType::Int(32));
        assert_eq!(parse_param_type("uint32").unwrap(), ParamType::Uint(32));
    }

    #[test]
    fn test_parse_array_types() {
        assert_eq!(
            parse_param_type("address[]").unwrap(),
            ParamType::Array(Box::new(ParamType::Address))
        );
        assert_eq!(
            parse_param_type("uint[]").unwrap(),
            ParamType::Array(Box::new(ParamType::Uint(256)))
        );
        assert_eq!(
            parse_param_type("bytes[]").unwrap(),
            ParamType::Array(Box::new(ParamType::Bytes))
        );
        assert_eq!(
            parse_param_type("bool[][]").unwrap(),
            ParamType::Array(Box::new(ParamType::Array(Box::new(ParamType::Bool))))
        );
    }

    #[test]
    fn test_parse_fixed_array_types() {
        assert_eq!(
            parse_param_type("address[2]").unwrap(),
            ParamType::FixedArray(Box::new(ParamType::Address), 2)
        );
        assert_eq!(
            parse_param_type("bool[17]").unwrap(),
            ParamType::FixedArray(Box::new(ParamType::Bool), 17)
        );
        assert_eq!(
            parse_param_type("bytes[45][3]").unwrap(),
            ParamType::FixedArray(
                Box::new(ParamType::FixedArray(Box::new(ParamType::Bytes), 45)),
                3
            )
        );
    }

    #[test]
    fn test_parse_mixed_array_types() {
        assert_eq!(
            parse_param_type("bool[][3]").unwrap(),
            ParamType::FixedArray(Box::new(ParamType::Array(Box::new(ParamType::Bool))), 3)
        );
        assert_eq!(
            parse_param_type("bool[3][]").unwrap(),
            ParamType::Array(Box::new(ParamType::FixedArray(
                Box::new(ParamType::Bool),
                3
            )))
        );
    }

    #[test]
    fn test_parse_tuple_types() {
        assert_eq!(
            parse_param_type("(address,bool)").unwrap(),
            ParamType::Tuple(vec![ParamType::Address, ParamType::Bool])
        );
        assert_eq!(
            parse_param_type("(bool[3],uint256)").unwrap(),
            ParamType::Tuple(vec![
                ParamType::FixedArray(Box::new(ParamType::Bool), 3),
                ParamType::Uint(256)
            ])
        );
    }

    #[test]
    fn test_parse_nested_tuple_types() {
        assert_eq!(
            parse_param_type("(address,bool,(bool,uint256))").unwrap(),
            ParamType::Tuple(vec![
                ParamType::Address,
                ParamType::Bool,
                ParamType::Tuple(vec![ParamType::Bool, ParamType::Uint(256)])
            ])
        );
        assert_eq!(
            parse_param_type("(address,bool,(bool,uint256,(bool,uint256)),(bool,uint256))")
                .unwrap(),
            ParamType::Tuple(vec![
                ParamType::Address,
                ParamType::Bool,
                ParamType::Tuple(vec![
                    ParamType::Bool,
                    ParamType::Uint(256),
                    ParamType::Tuple(vec![ParamType::Bool, ParamType::Uint(256)])
                ]),
                ParamType::Tuple(vec![ParamType::Bool, ParamType::Uint(256)])
            ])
        );
    }

    #[test]
    fn test_parse_tuple_array_types() {
        assert_eq!(
            parse_param_type("(uint256,bytes32)[]").unwrap(),
            ParamType::Array(Box::new(ParamType::Tuple(vec![
                ParamType::Uint(256),
                ParamType::FixedBytes(32)
            ])))
        )
    }

    #[test]
    fn test_parse_nested_tuple_array_types() {
        assert_eq!(
            parse_param_type("((uint256,bytes32)[],address)").unwrap(),
            ParamType::Tuple(vec![
                ParamType::Array(Box::new(ParamType::Tuple(vec![
                    ParamType::Uint(256),
                    ParamType::FixedBytes(32),
                ]))),
                ParamType::Address,
            ])
        );
    }

    #[test]
    fn test_garbage_after_type() {
        assert_eq!(
            parse_param_type("address[]()").unwrap_err(),
            ParseError::Syntax(nom::error::Error {
                input: "()",
                code: nom::error::ErrorKind::Eof
            })
        );
    }
}
