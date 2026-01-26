pub enum HeadingLevel {
    h1,
    h2,
    h3,
    h4,
    h5,
    h6
}


pub struct Section {
    level: HeadingLevel,
    title: String,
    content: Vec<Renderable>,
}

impl Renderable for Prose {
    fn render() -> String {
        todo!()
    }
}

