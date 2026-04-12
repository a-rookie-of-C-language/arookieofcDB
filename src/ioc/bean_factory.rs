use super::bean_definition::{BeanDefinition,SharedBean};

pub trait BeanFactory: Send + Sync {
    fn get_bean(&self, name: &str) -> Option<SharedBean>;
    fn is_singleton(&self, name: &str) -> bool;
    fn contains_bean(&self, name: &str) -> bool;
    fn do_create_bean(&self, name: &str) -> Option<SharedBean>;
}

pub trait BeanDefinitionRegistry: Send + Sync {
    fn register_bean_definition(&mut self, name: &str, bean_definition: Box<dyn BeanDefinition>);
    fn remove_bean_definition(&mut self, bean_name: &str);
    fn contains_bean_definition(&self, bean_name: &str) -> bool;
    fn get_bean_definition(&self, bean_name: &str) -> Option<&dyn BeanDefinition>;
    fn get_bean_definition_names(&self) -> Vec<String>;
    fn get_bean_definition_count(&self) -> usize;
    fn is_bean_name_in_use(&self, bean_name: &str) -> bool;
}
