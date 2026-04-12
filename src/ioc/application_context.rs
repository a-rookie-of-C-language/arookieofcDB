use super::bean_definition::SharedBean;

pub trait ApplicationContext: Send + Sync {
    fn get_bean(&self, name: &str) -> Option<SharedBean>;
    fn is_singleton(&self, name: &str) -> bool;
    fn contains_bean(&self, name: &str) -> bool;
    fn do_create_bean(&self, name: &str) -> Option<SharedBean>;
}

pub trait ConfigurableApplicationContext: ApplicationContext {
    fn refresh(&mut self);
    fn close(&mut self);
    fn is_active(&self) -> bool;
}
