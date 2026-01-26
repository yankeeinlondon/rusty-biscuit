


/// The **Compose** struct allows you to _compose_
/// 1 or more _renderable_ components together.
pub struct Compose {
    parts: Vec<Renderable>
}

impl Renderable for Compose {
    fn render() -> String {
        todo!()
    }
}

impl Compose {
    pub fn add_prose(self, content: Prose) -> self {
        todo!()
    }

    pub fn add_table<T:Into<String>>(self, content: Table) -> self {
        todo!()
    }

    pub fn add_unordered_list(self, content: UnorderedList) -> self {
        todo!()
    }

    pub fn add_ordered_list(self, content: UnorderedList) -> self {
        todo!()
    }

}
