use super::application_context::{ApplicationContext, ConfigurableApplicationContext};
use super::bean_definition::{BeanDefinition, BeanScope, SharedBean};
use super::bean_factory::{BeanDefinitionRegistry, BeanFactory};
use super::default_listable_bean_factory::DefaultListableBeanFactory;

pub struct AbstractApplicationContext {
    bean_factory: DefaultListableBeanFactory,
}

impl Default for AbstractApplicationContext {
    fn default() -> Self {
        Self {
            bean_factory: DefaultListableBeanFactory::new(),
        }
    }
}

impl AbstractApplicationContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_environment(&mut self, environment: std::collections::HashMap<String, String>) {
        self.bean_factory.set_environment(environment);
    }

    pub fn get_environment(&self) -> std::collections::HashMap<String, String> {
        self.bean_factory.get_environment()
    }
}

impl ConfigurableApplicationContext for AbstractApplicationContext {
    fn refresh(&mut self) {
        let names = BeanDefinitionRegistry::get_bean_definition_names(&self.bean_factory);
        for name in names {
            if let Some(definition) = self.bean_factory.get_bean_definition(&name) {
                if !definition.is_lazy_init() && definition.get_scope() == BeanScope::Singleton {
                    self.bean_factory.do_create_bean(&name);
                }
            }
        }
    }

    fn close(&mut self) {
        self.bean_factory.destroy_singletons();
    }

    fn is_active(&self) -> bool {
        true
    }
}

impl ApplicationContext for AbstractApplicationContext {
    fn contains_bean(&self, name: &str) -> bool {
        self.bean_factory.contains_bean(name)
    }

    fn do_create_bean(&self, name: &str) -> Option<SharedBean> {
        self.bean_factory.do_create_bean(name)
    }

    fn get_bean(&self, name: &str) -> Option<SharedBean> {
        self.bean_factory.get_bean(name)
    }

    fn is_singleton(&self, name: &str) -> bool {
        self.bean_factory.is_singleton(name)
    }
}

impl BeanDefinitionRegistry for AbstractApplicationContext {
    fn register_bean_definition(&mut self, name: &str, bean_definition: Box<dyn BeanDefinition>) {
        self.bean_factory.register_bean_definition(name, bean_definition);
    }

    fn remove_bean_definition(&mut self, bean_name: &str) {
        self.bean_factory.remove_bean_definition(bean_name);
    }

    fn contains_bean_definition(&self, bean_name: &str) -> bool {
        self.bean_factory.contains_bean_definition(bean_name)
    }

    fn get_bean_definition(&self, bean_name: &str) -> Option<&dyn BeanDefinition> {
        self.bean_factory.get_bean_definition(bean_name)
    }

    fn get_bean_definition_names(&self) -> Vec<String> {
        BeanDefinitionRegistry::get_bean_definition_names(&self.bean_factory)
    }

    fn get_bean_definition_count(&self) -> usize {
        self.bean_factory.get_bean_definition_count()
    }

    fn is_bean_name_in_use(&self, bean_name: &str) -> bool {
        self.bean_factory.is_bean_name_in_use(bean_name)
    }
}
