use super::root_bean_definition::RootBeanDefinition;

pub struct BeanRegistration {
    pub definition: fn() -> RootBeanDefinition,
}

inventory::collect!(BeanRegistration);
