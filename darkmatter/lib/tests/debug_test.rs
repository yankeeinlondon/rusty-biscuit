#[test]
fn test_pulldown_cmark_behavior() {
    use pulldown_cmark::Parser;

    let input = "---";
    let parser = Parser::new(input);
    let events: Vec<_> = parser.collect();
    println!("Events for '---': {:?}", events);

    let input2 = "--- { style: waves }";
    let parser2 = Parser::new(input2);
    let events2: Vec<_> = parser2.collect();
    println!("Events for '--- {{ style: waves }}': {:?}", events2);
}
