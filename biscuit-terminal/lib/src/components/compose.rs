use crate::components::{
    list::{OrderedList, UnorderedList}, prose::Prose, renderable::RenderableContent
};




/// The **Compose** struct allows you to _compose_
/// 1 or more _renderable_ components together.
pub struct Compose {
    parts: Vec<RenderableContent>
}

impl From<String> for Compose {
    fn from(value: String) -> Self {
        Compose {
            parts: vec![RenderableContent::String(value)]
        }
    }
}

impl From<&str> for Compose {
    fn from(value: &str) -> Self {
        Compose {
            parts: vec![RenderableContent::String(value.into())]
        }
    }
}

impl From<RenderableContent> for Compose {
    fn from(value: RenderableContent) -> Self {
        Compose {
            parts: vec![value]
        }
    }
}


impl Compose {

    /// Adds a block of _prose_ which is text that is allowed
    /// to embed styling tokens in it that can be rendered lazily
    /// when we're ready to send to the terminal.
    pub fn add_prose(&mut self, content: Prose) -> &Self {
        self.parts.push(RenderableContent::from(content) );

        self
    }

    pub fn add_text<T: Into<String>>(&mut self, content: T) -> &Self {
        let text = content.into();
        self.parts.push(RenderableContent::from(text));

        self
    }

    // pub fn add_table<T:Into<String>>(self, content: Table) -> self {
    //     todo!()
    // }

    pub fn add_unordered_list(&mut self, content: UnorderedList) -> &Self {
        self.parts.push(RenderableContent::from(content));

        self
    }

    pub fn add_ordered_list(&mut self, content: OrderedList) -> &Self {
        self.parts.push(RenderableContent::from(content));

        self
    }

}
