use crate::page::FullPage;

pub struct Router {
    pub(crate) routes: Vec<Box<dyn FullPage>>,
}

impl Router {
    pub fn new(routes: Vec<Box<dyn FullPage>>) -> Self {
        Router { routes }
    }
}
