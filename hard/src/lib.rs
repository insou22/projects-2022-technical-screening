use std::collections::BTreeMap;
use std::io::BufReader;
use std::fs::File;

use nom::{IResult, AsChar};
use nom::error::ErrorKind;
use nom::sequence::tuple;
use nom::combinator::{
    opt,
    map, eof,
};
use nom::character::complete::{space0, space1, digit1};
use nom::multi::separated_list1;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_while_m_n};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use strsim::normalized_damerau_levenshtein;

#[pyfunction]
pub fn is_unlocked(transcript: Vec<String>, course: String) -> PyResult<bool> {
    let transcript = transcript
        .into_iter()
        .map(|string| string.to_ascii_lowercase())
        .collect::<Vec<_>>();

    let conditions: BTreeMap<String, String> = serde_json::from_reader(
        BufReader::new(File::open("./conditions.json").unwrap())
    ).unwrap();

    let target_condition = conditions.get(&course)
        .ok_or_else(|| PyValueError::new_err(format!("Failed to find condition for course {}", course)))?;

    let course = course.to_ascii_lowercase();

    let condition = target_condition.to_ascii_lowercase();
    let (_leftover, condition) = parse(&condition)
        .map_err(|err| PyValueError::new_err(format!("Failed to parse condition for course ({}): {}", course, err)))?;

    Ok(evaluate_condition(&transcript, &course, &condition))
}

fn evaluate_condition(transcript: &[String], target_course: &str, condition: &Condition) -> bool {
    use Condition::*;
    
    match condition {
        Empty => true,
        Course(course) => transcript.contains(course),
        ImpliedCourseCode(code) => transcript.contains(&format!("{}{}", &target_course[..4], code)),
        And(cond1, cond2) => evaluate_condition(transcript, target_course, cond1) && evaluate_condition(transcript, target_course, cond2),
        Or(cond1, cond2) => evaluate_condition(transcript, target_course, cond1) || evaluate_condition(transcript, target_course, cond2),
        Uoc(uoc, category) => {
            use Category::*;

            let uoc = *uoc;
            
            // assume all courses in transcript are 6uoc,
            // for the purposes of the exercise
            match category {
                Some(Comp) => {
                    transcript.iter()
                        .filter(|course| course.starts_with("comp"))
                        .count() as u32 * 6 >= uoc
                }
                Some(CompLevel(level)) => {
                    transcript.iter()
                        .filter(|course| course.starts_with(&format!("comp{}", level)))
                        .count() as u32 * 6 >= uoc
                }
                Some(Courses(courses)) => {
                    transcript.iter()
                        .filter(|course| courses.contains(course))
                        .count() as u32 * 6 >= uoc
                }
                None => {
                    transcript.len() as u32 * 6 >= uoc
                }
            }
        }
    }
}

#[derive(Debug)]
enum Condition {
    Empty,
    Course(String),
    ImpliedCourseCode(String),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Uoc(u32, Option<Category>),
}

#[derive(Debug)]
enum Category {
    Comp,
    CompLevel(u32),
    Courses(Vec<String>),
}

// let's assume UTF-8 compliance, for sanity's sake
fn parse(input: &str) -> IResult<&str, Condition> {
    map(
        tuple((
            parse_prereq_header,
            parse_condition,
        )),
        |(
            _header,
            condition,
        )| condition,
    )(input)
}

fn parse_condition(input: &str) -> IResult<&str, Condition> {
    alt((
        parse_or_condition,     // seemingly don't need to worry
        parse_and_condition,    // about associativity for this exercise
        parse_base_condition,
    ))(input)
}

fn parse_base_condition(input: &str) -> IResult<&str, Condition> {
    alt((
        parse_empty_condition,
        parse_units_of_credit,
        map(
            parse_course,
            |course| Condition::Course(course),
        ),
        parse_parenthesised_condition,
        parse_implied_course_code,
    ))(input)
}

fn parse_empty_condition(input: &str) -> IResult<&str, Condition> {
    map(
        eof,
        |_| Condition::Empty,
    )(input)
}

fn parse_or_condition(input: &str) -> IResult<&str, Condition> {
    map(
        tuple((
            parse_base_condition,
            space0,
            tag("or"),
            space1,
            parse_condition,
        )),
        |(
            cond1,
            _,
            _,
            _,
            cond2,
        )| Condition::Or(Box::new(cond1), Box::new(cond2)),
    )(input)
}

fn parse_and_condition(input: &str) -> IResult<&str, Condition> {
    map(
        tuple((
            parse_base_condition,
            space0,
            tag("and"),
            space1,
            parse_condition,
        )),
        |(
            cond1,
            _,
            _,
            _,
            cond2,
        )| Condition::And(Box::new(cond1), Box::new(cond2)),
    )(input)
}

fn parse_units_of_credit(input: &str) -> IResult<&str, Condition> {
    map(
        tuple((
            opt(
                tuple((
                    typo_tag("completion", is_space),
                    space1,
                    typo_tag("of", is_space),
                ))
            ),
            space0,
            digit1,
            space0,
            typo_tag("units", is_space),
            space1,
            typo_tag_with_dist("of", 0.50, is_space),
            space1,
            typo_tag("credit", is_space),
            space0,
            opt(
                tuple((
                    typo_tag("in", is_space),
                    space1,
                    parse_category,
                ))
            ),
        )),
        |(
            _,
            _,
            amount,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            category,
        )| Condition::Uoc(amount.parse::<u32>().unwrap(), category.map(|(_, _, category)| category)),
    )(input)
}

fn parse_parenthesised_condition(input: &str) -> IResult<&str, Condition> {
    map(
        tuple((
            tag("("),
            parse_condition,
            tag(")"),
        )),
        |(_, condition, _)| condition,
    )(input)
}

fn parse_category(input: &str) -> IResult<&str, Category> {
    alt((
        parse_level_category,
        parse_list_category,
        parse_comp_category,
    ))(input)
}

fn parse_level_category(input: &str) -> IResult<&str, Category> {
    map(
        tuple((
            typo_tag("level", is_space),
            space1,
            digit1,
            space1,
            typo_tag("comp", is_space),
            space1,
            typo_tag("courses", is_not_alpha),
        )),
        |(
            _,
            _,
            level,
            ..
        )| Category::CompLevel(level.parse::<u32>().unwrap())
    )(input)
}

fn parse_list_category(input: &str) -> IResult<&str, Category> {
    map(
        tuple((
            tag("("),
            separated_list1(
                tuple((
                    space0,
                    tag(","),
                    space0,
                )),
                parse_course,    
            ),
            tag(")"),
        )),
        |(
            _,
            courses,
            _,
        )| Category::Courses(courses)
    )(input)
}

fn parse_comp_category(input: &str) -> IResult<&str, Category> {
    map(
        tuple((
            typo_tag("comp", is_not_alpha),
            space1,
            typo_tag("courses", is_not_alpha)
        )),
        |_| Category::Comp,
    )(input)
}

fn parse_prereq_header(input: &str) -> IResult<&str, ()> {
    map(
        tuple((
            opt(
                alt((
                    typo_tag("prerequisite", is_colon_or_space),
                    typo_tag("prereq",       is_colon_or_space),
                ))
            ),
            opt(tag(":")),
            space0,
        )),
        |_| (), 
    )(input)
}

fn parse_implied_course_code(input: &str) -> IResult<&str, Condition> {
    map(
        take_while_m_n(4, 4, AsChar::is_dec_digit),
        |code: &str| Condition::ImpliedCourseCode(code.to_string()),
    )(input)
}

fn parse_course(input: &str) -> IResult<&str, String> {
    map(
        tuple((
            take_while_m_n(4, 4, AsChar::is_alpha),
            take_while_m_n(4, 4, AsChar::is_dec_digit),
        )),
        |(faculty, number)| format!("{}{}", faculty, number),
    )(input)
}

const DEFAULT_DAMERAU_LEVENSHTEIN_MIN_DIST: f64 = 0.80;

fn typo_tag<F>(tag: &str, until: F) -> impl Fn(&str) -> IResult<&str, &str>
where
    F: Fn(char) -> bool,
{
    typo_tag_with_dist(tag, DEFAULT_DAMERAU_LEVENSHTEIN_MIN_DIST, until)
}

fn typo_tag_with_dist<F>(tag: &str, dist: f64, until: F) -> impl Fn(&str) -> IResult<&str, &str>
where
    F: Fn(char) -> bool,
{
    let tag = tag.to_string();

    move |input| {
        let parsed: (&str, &str) = take_till(&until)(input)?;
        if normalized_damerau_levenshtein(parsed.1, &tag) >= dist {
            Ok(parsed)
        } else {
            Err(nom::Err::Error(nom::error::Error::new(input, ErrorKind::Tag)))
        }
    }
}

fn is_colon_or_space(char: char) -> bool {
    char == ':' || char.is_ascii_whitespace()
}

fn is_space(char: char) -> bool {
    char.is_ascii_whitespace()
}

fn is_not_alpha(char: char) -> bool {
    !char.is_alpha()
}

#[pymodule]
fn hard(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(is_unlocked, m)?)?;
    Ok(())
}
