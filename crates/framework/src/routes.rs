use crate::page::FullPage;

pub struct Router<'a> {
    pub(crate) routes: Vec<&'a dyn FullPage>,
}

impl<'a> Router<'a> {
    pub fn new(routes: Vec<&'a dyn FullPage>) -> Router<'a> {
        Router { routes }
    }
}
