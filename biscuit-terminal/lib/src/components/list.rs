use crate::components::{
    renderable::{Renderable, RenderableContent}
};


#[derive(Debug)]
pub struct OrderedList {
    items: Vec<RenderableContent>
}

impl<T: Into<String>> From<Vec<T>> for OrderedList {
    fn from(value: Vec<T>) -> Self {
        OrderedList {
            items: value.into_iter().map(|f| RenderableContent::String(f.into())).collect(),
        }
    }
}

impl From<Vec<RenderableContent>> for OrderedList {
    fn from(value: Vec<RenderableContent>) -> Self {
        OrderedList {
            items: value
        }
    }
}

impl From<Vec<&RenderableContent>> for OrderedList {
    fn from(value: Vec<&RenderableContent>) -> Self {
        OrderedList {
            items: value.into_iter().cloned().collect(),
        }
    }
}

impl Renderable for OrderedList {
    fn render(&self, _layout: Option<&crate::utils::layout::Layout>) -> String {
        todo!()
    }

    fn fallback_render(&self, _term: &crate::terminal::Terminal, _layout: Option<&crate::utils::layout::Layout>) -> String {
        todo!()
    }
}



#[derive(Debug)]
pub struct UnorderedList {
    items: Vec<RenderableContent>
}

impl<T: Into<String>> From<Vec<T>> for UnorderedList {
    fn from(value: Vec<T>) -> Self {
        UnorderedList {
            items: value.into_iter().map(|f| RenderableContent::String(f.into())).collect(),
        }
    }
}

impl From<Vec<&RenderableContent>> for UnorderedList {
    fn from(value: Vec<&RenderableContent>) -> Self {
        UnorderedList {
            items: value.into_iter().cloned().collect(),
        }
    }
}

impl Renderable for UnorderedList {
    fn render(&self, layout: Option<&crate::utils::layout::Layout>) -> String {
        todo!()
    }

    fn fallback_render(&self, term: &crate::terminal::Terminal, layout: Option<&crate::utils::layout::Layout>) -> String {
        todo!()
    }
}

