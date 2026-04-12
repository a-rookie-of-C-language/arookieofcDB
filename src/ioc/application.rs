use std::sync::OnceLock;

use super::abstract_application_context::AbstractApplicationContext;
use super::application_context::{ApplicationContext, ConfigurableApplicationContext};
use super::bean_definition::{BeanDefinition, SharedBean};
use super::bean_factory::BeanDefinitionRegistry;
use super::registry::BeanRegistration;

static CONTEXT: OnceLock<AbstractApplicationContext> = OnceLock::new();

pub struct Application;

impl Application {
    pub fn run() {
        let mut context = AbstractApplicationContext::new();

        let environment: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for registration in inventory::iter::<BeanRegistration> {
            let definition = (registration.definition)();

            if let Some((key, expected)) = definition.get_condition() {
                let actual = environment.get(key).map(|s| s.as_str()).unwrap_or("");
                if actual != expected {
                    continue;
                }
            }

            let name = definition.get_bean_class_name().to_string();
            context.register_bean_definition(&name, Box::new(definition));
        }

        context.set_environment(environment);
        context.refresh();

        if CONTEXT.set(context).is_err() {
            panic!("Application context already initialized");
        }
    }

    pub fn get_bean(name: &str) -> Option<SharedBean> {
        CONTEXT.get()?.get_bean(name)
    }

    pub fn contains_bean(name: &str) -> bool {
        CONTEXT
            .get()
            .map(|c| c.contains_bean(name))
            .unwrap_or(false)
    }

    pub fn is_singleton(name: &str) -> bool {
        CONTEXT.get().map(|c| c.is_singleton(name)).unwrap_or(false)
    }
}
